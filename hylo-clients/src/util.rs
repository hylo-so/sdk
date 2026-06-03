use std::iter::once;

use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::address_lookup_table::state::AddressLookupTable;
use anchor_client::solana_sdk::address_lookup_table::AddressLookupTableAccount;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::hash::Hash;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::message::{v0, VersionedMessage};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::solana_sdk::transaction::VersionedTransaction;
use anchor_client::solana_sdk::{bs58, pubkey};
use anchor_client::Cluster;
use anchor_lang::prelude::AccountMeta;
use anchor_lang::{AnchorDeserialize, Discriminator};
use anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use anchor_spl::token;
use anyhow::{anyhow, bail, Context, Result};
use hylo_core::idl::tokens::{StakePool, HYLOSOL, JITOSOL};
use itertools::Itertools;
use solana_rpc_client_api::config::RpcSimulateTransactionConfig;
use solana_rpc_client_api::response::{Response, RpcSimulateTransactionResult};
use solana_transaction_status_client_types::{
  UiInstruction, UiParsedInstruction, UiPartiallyDecodedInstruction,
};

use crate::earn_pool_client::EarnPoolClient;
use crate::exchange_client::ExchangeClient;
use crate::prelude::VersionedTransactionData;
use crate::program_client::ProgramClient;
use crate::router_client::RouterClient;

pub trait LST: StakePool {}
impl LST for JITOSOL {}
impl LST for HYLOSOL {}

#[cfg(not(feature = "shadow"))]
pub const HYLO_LOOKUP_TABLE: Pubkey =
  pubkey!("71Upv8sJ7wtMpX95ndwVWJvCG3QtpDdVrsh2uJXJvtUz");
#[cfg(feature = "shadow")]
pub const HYLO_LOOKUP_TABLE: Pubkey =
  pubkey!("AUJBw5F13K3pSJMZWok21xKueaNR9N5gM3r3ZFyx7det");

#[cfg(not(feature = "shadow"))]
pub const LST_REGISTRY_LOOKUP_TABLE: Pubkey =
  pubkey!("9Mb2Mt76AN7eNY3BBA4LgfTicARXhcEEokTBfsN47noK");
#[cfg(feature = "shadow")]
pub const LST_REGISTRY_LOOKUP_TABLE: Pubkey =
  pubkey!("CoBiwzy3VjtXumzT4YsGZb7mQKRrwkpkeixsvnfEEeL4");

/// This wallet should hold at least one unit of jitoSOL, xSOL, hyUSD, and
/// sHYUSD. Useful for simulations of mint and redemption.
#[cfg(not(feature = "shadow"))]
pub const REFERENCE_WALLET: Pubkey =
  pubkey!("GUX587fnbnZmqmq2hnav8r6siLczKS8wrp9QZRhuWeai");
#[cfg(feature = "shadow")]
pub const REFERENCE_WALLET: Pubkey =
  pubkey!("EvSpFLUfdJT38di12JhAwfU6xe6YCKS6BL1gN3VDWYQG");

/// Default configuration to use in simulated transactions.
#[must_use]
pub fn simulation_config() -> RpcSimulateTransactionConfig {
  RpcSimulateTransactionConfig {
    sig_verify: false,
    replace_recent_blockhash: true,
    commitment: Some(CommitmentConfig::confirmed()),
    inner_instructions: true,
    ..Default::default()
  }
}

/// Parses a typed event from simulation inner instructions.
///
/// # Errors
/// * Simulation failed
/// * No inner instructions returned
/// * Event not found or deserialization fails
pub fn parse_event<E>(
  result: &Response<RpcSimulateTransactionResult>,
) -> Result<E>
where
  E: AnchorDeserialize + Discriminator,
{
  if let Some(err) = &result.value.err {
    bail!("Simulation failed: {err:?}")
  } else if let Some(ixs) = &result.value.inner_instructions {
    ixs
      .iter()
      .flat_map(|ix| &ix.instructions)
      .find_map(|ix| match ix {
        UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(
          UiPartiallyDecodedInstruction { data, .. },
        )) => bs58::decode(data).into_vec().ok(),
        _ => None,
      })
      .filter(|bytes| bytes.len() >= 16 && &bytes[8..16] == E::DISCRIMINATOR)
      .context("Could not parse event from result")
      .and_then(|bytes| Ok(E::try_from_slice(&bytes[16..])?))
  } else {
    bail!("Simulation succeeded but no inner instructions returned")
  }
}

/// Like [`parse_event`], but filters inner instructions by event
/// discriminator BEFORE picking. Safe to call multiple times on the same
/// simulation result with different `E` to extract distinct events from
/// the same tx (e.g., a trigger-orders fill event + its inner Hylo
/// convert event).
///
/// `parse_event` takes the FIRST `PartiallyDecoded` inner instruction and
/// only then checks the discriminator, so it cannot extract a second,
/// differently-typed event from the same result. This variant filters by
/// `E::DISCRIMINATOR` first, then picks the first match.
///
/// Note: an instruction whose discriminator matches `E::DISCRIMINATOR` but
/// whose payload fails to deserialize is SKIPPED (not surfaced as an error),
/// unlike `parse_event`, which propagates the borsh error.
///
/// # Errors
/// * Simulation contains a tx-level error
/// * No inner instructions returned
/// * No event matching `E::DISCRIMINATOR` found, or deserialization fails
pub fn parse_event_filtered<E>(
  result: &Response<RpcSimulateTransactionResult>,
) -> Result<E>
where
  E: AnchorDeserialize + Discriminator,
{
  if let Some(err) = &result.value.err {
    bail!("Simulation failed: {err:?}")
  }
  let Some(ixs) = &result.value.inner_instructions else {
    bail!("Simulation succeeded but no inner instructions returned")
  };
  ixs
    .iter()
    .flat_map(|ix| &ix.instructions)
    .filter_map(|ix| match ix {
      UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(
        UiPartiallyDecodedInstruction { data, .. },
      )) => bs58::decode(data).into_vec().ok(),
      _ => None,
    })
    .filter(|bytes| bytes.len() >= 16 && &bytes[8..16] == E::DISCRIMINATOR)
    .find_map(|bytes| E::try_from_slice(&bytes[16..]).ok())
    .context("Could not parse event from result")
}

/// Deserializes an account into an address lookup table.
///
/// # Errors
/// * Account data cannot be deserialized
pub fn deserialize_lookup_table(
  key: &Pubkey,
  account: &Account,
) -> Result<AddressLookupTableAccount> {
  let table = AddressLookupTable::deserialize(&account.data)?;
  Ok(AddressLookupTableAccount {
    key: *key,
    addresses: table.addresses.to_vec(),
  })
}

/// Builds a signed versioned transaction.
///
/// # Errors
/// * Failed to compile message
/// * Failed to create transaction
pub fn build_v0_transaction(
  VersionedTransactionData {
    instructions,
    lookup_tables,
  }: &VersionedTransactionData,
  payer: &Keypair,
  additional_signers: &[&Keypair],
  recent_blockhash: Hash,
) -> Result<VersionedTransaction> {
  let message = v0::Message::try_compile(
    &payer.pubkey(),
    instructions,
    lookup_tables,
    recent_blockhash,
  )?;
  let signatures = once(payer)
    .chain(additional_signers.iter().copied())
    .map(|signer| signer.sign_message(&message.serialize()))
    .collect_vec();
  let tx = VersionedTransaction {
    message: VersionedMessage::V0(message),
    signatures,
  };
  Ok(tx)
}

/// Creates `remaining_accounts` array from LST registry table with all
/// headers writable.
///
/// # Errors
/// * Lookup table account doesn't exist
/// * Malformed structure (preamble cannot be split at 16)
pub fn build_lst_registry(
  table: AddressLookupTableAccount,
) -> Result<(Vec<AccountMeta>, AddressLookupTableAccount)> {
  if let Some((preamble, blocks)) = table.addresses.split_at_checked(16) {
    let preamble = preamble
      .iter()
      .map(|key| AccountMeta::new_readonly(*key, false));
    let blocks =
      blocks
        .iter()
        .tuples()
        .flat_map(|(header, mint, vault, pool_state)| {
          [
            AccountMeta::new(*header, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*vault, false),
            AccountMeta::new_readonly(*pool_state, false),
          ]
        });
    let remaining_accounts = preamble.chain(blocks).collect_vec();
    Ok((remaining_accounts, table))
  } else {
    Err(anyhow!("Malformed LST registry preamble."))
  }
}

/// Gets cluster from environment variables.
///
/// # Errors
/// * Missing `RPC_URL` or `RPC_WS_URL` environment variables
pub fn cluster_from_env() -> Result<Cluster> {
  let url = std::env::var("RPC_URL")?;
  let ws_url = std::env::var("RPC_WS_URL")?;
  Ok(Cluster::Custom(url, ws_url))
}

/// Builds test exchange client with random keypair.
///
/// # Errors
/// * Environment variable access
/// * Client initialization
pub fn build_test_exchange_client() -> Result<ExchangeClient> {
  let client = ExchangeClient::new_from_keypair(
    cluster_from_env()?,
    Keypair::new(),
    CommitmentConfig::confirmed(),
  )?;
  Ok(client)
}

/// Builds test earn pool client with random keypair.
///
/// # Errors
/// * Environment variable access
/// * Client initialization
pub fn build_test_earn_pool_client() -> Result<EarnPoolClient> {
  let client = EarnPoolClient::new_from_keypair(
    cluster_from_env()?,
    Keypair::new(),
    CommitmentConfig::confirmed(),
  )?;
  Ok(client)
}

/// Builds test router client with random keypair.
///
/// # Errors
/// * Environment variable access
/// * Client initialization
pub fn build_test_router_client() -> Result<RouterClient> {
  RouterClient::new_from_keypair(
    cluster_from_env()?,
    Keypair::new(),
    CommitmentConfig::confirmed(),
  )
}

/// Builds ATA creation instruction for a user and mint.
#[must_use]
pub fn user_ata_instruction(user: &Pubkey, mint: &Pubkey) -> Instruction {
  create_associated_token_account_idempotent(user, user, mint, &token::ID)
}

#[cfg(test)]
mod parse_event_filtered_tests {
  use anchor_lang::{AnchorSerialize, Discriminator};
  use hylo_idl::exchange::events::ConvertStableToLeverLstEvent;
  use hylo_idl::trigger_orders::events::TriggerOrderFilledEvent;
  use hylo_idl::trigger_orders::types::{
    ConvertDirection, PairTarget, TriggerDirection,
  };
  use solana_rpc_client_api::response::{
    RpcResponseContext, RpcSimulateTransactionResult,
  };
  use solana_transaction_status_client_types::UiInnerInstructions;

  use super::*;

  /// Encodes an event the way an Anchor event-CPI inner instruction carries
  /// it on the wire: `[8-byte self-CPI discriminator][8-byte event
  /// discriminator][borsh(event)]`, then base58 (as the JSON RPC returns it).
  fn encode_event<E: AnchorSerialize + Discriminator>(event: &E) -> String {
    // The first 8 bytes are the Anchor event-CPI self-discriminator. Their
    // exact value is irrelevant to parsing (`parse_event*` only reads
    // `bytes[8..16]` for the event discriminator), so we use a sentinel.
    let mut bytes = vec![0xAB_u8; 8];
    bytes.extend_from_slice(E::DISCRIMINATOR);
    bytes.extend_from_slice(&event.try_to_vec().expect("borsh serialize"));
    bs58::encode(bytes).into_string()
  }

  fn partially_decoded(data: String) -> UiInstruction {
    UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(
      UiPartiallyDecodedInstruction {
        program_id: Pubkey::new_unique().to_string(),
        accounts: vec![],
        data,
        stack_height: None,
      },
    ))
  }

  /// Builds a simulation response carrying the two given inner-instruction
  /// data blobs, mirroring a real `execute_order_*` tx that emits both a
  /// `TriggerOrderFilledEvent` and an inner Hylo convert event.
  fn response_with(
    blobs: Vec<String>,
  ) -> Response<RpcSimulateTransactionResult> {
    Response {
      context: RpcResponseContext {
        slot: 0,
        api_version: None,
      },
      value: RpcSimulateTransactionResult {
        err: None,
        logs: None,
        accounts: None,
        units_consumed: None,
        loaded_accounts_data_size: None,
        return_data: None,
        inner_instructions: Some(vec![UiInnerInstructions {
          index: 0,
          instructions: blobs.into_iter().map(partially_decoded).collect(),
        }]),
        replacement_blockhash: None,
      },
    }
  }

  fn sample_trigger_order_filled() -> TriggerOrderFilledEvent {
    TriggerOrderFilledEvent {
      order: Pubkey::new_unique(),
      owner: Pubkey::new_unique(),
      executor: Pubkey::new_unique(),
      pair_target: PairTarget::Lst,
      convert_direction: ConvertDirection::StableToLever,
      nonce: 42,
      escrow_spent: 1_000,
      output_received: hylo_idl::trigger_orders::types::UFixValue64 {
        bits: 2_000,
        exp: -6,
      },
      output_mint: Pubkey::new_unique(),
      trigger_price: 100,
      trigger_expo: -8,
      direction: TriggerDirection::AtOrAbove,
      pyth_price: 123,
      pyth_expo: -8,
    }
  }

  fn sample_convert_event() -> ConvertStableToLeverLstEvent {
    use hylo_idl::exchange::types::UFixValue64;
    ConvertStableToLeverLstEvent {
      stablecoin_burned: UFixValue64 { bits: 10, exp: -6 },
      stablecoin_fees: UFixValue64 { bits: 1, exp: -6 },
      stablecoin_nav: UFixValue64 { bits: 100, exp: -6 },
      levercoin_minted: UFixValue64 { bits: 5, exp: -6 },
      levercoin_nav: UFixValue64 { bits: 200, exp: -6 },
    }
  }

  // The crux of Task 17: a single `execute_order_*` simulation result carries
  // TWO differently-typed events. `parse_event_filtered` must extract BOTH
  // from the SAME result by filtering on the discriminator before picking —
  // something the existing `parse_event` (first-ix-then-filter) cannot do.
  #[test]
  fn extracts_two_distinct_events_from_one_result() {
    let filled = sample_trigger_order_filled();
    let convert = sample_convert_event();
    let resp =
      response_with(vec![encode_event(&filled), encode_event(&convert)]);

    let got_filled: TriggerOrderFilledEvent =
      parse_event_filtered(&resp).expect("TriggerOrderFilledEvent extractable");
    let got_convert: ConvertStableToLeverLstEvent =
      parse_event_filtered(&resp).expect("convert event extractable");

    // Assert on distinguishing fields (generated event structs do not
    // necessarily derive PartialEq/Debug, so compare field-by-field).
    assert_eq!(got_filled.nonce, 42);
    assert_eq!(got_filled.escrow_spent, 1_000);
    assert_eq!(got_filled.output_received.bits, 2_000);
    assert_eq!(got_convert.stablecoin_burned.bits, 10);
    assert_eq!(got_convert.levercoin_nav.bits, 200);
  }

  // Order-independence: extraction works regardless of which event appears
  // first in the inner-instruction list (the inner convert event precedes
  // the outer fill event in the on-wire CPI order).
  #[test]
  fn extracts_when_target_event_is_not_first() {
    let filled = sample_trigger_order_filled();
    let convert = sample_convert_event();
    // Convert event first, fill event second.
    let resp =
      response_with(vec![encode_event(&convert), encode_event(&filled)]);

    let got_filled: TriggerOrderFilledEvent =
      parse_event_filtered(&resp).expect("fill event still extractable");
    assert_eq!(got_filled.nonce, 42);
  }

  #[test]
  fn errors_when_event_absent() {
    let convert = sample_convert_event();
    let resp = response_with(vec![encode_event(&convert)]);
    // No TriggerOrderFilledEvent present → must error, not silently succeed.
    let result: Result<TriggerOrderFilledEvent> = parse_event_filtered(&resp);
    assert!(result.is_err());
  }

  /// A discriminator-MATCHING but undeserializable blob: the correct event
  /// discriminator followed by a payload too short to borsh-decode (the first
  /// field of `TriggerOrderFilledEvent` is a 32-byte `Pubkey`, so 3 bytes
  /// fail).
  fn corrupt_blob<E: Discriminator>() -> String {
    let mut bytes = vec![0xAB_u8; 8];
    bytes.extend_from_slice(E::DISCRIMINATOR);
    bytes.extend_from_slice(&[0x00, 0x01, 0x02]);
    bs58::encode(bytes).into_string()
  }

  // `parse_event_filtered` must SKIP an inner instruction whose discriminator
  // matches `E` but whose payload fails to deserialize, and keep scanning for
  // a valid match — the documented divergence from `parse_event` (which
  // propagates the borsh error). Here a corrupt match precedes a valid one.
  #[test]
  fn skips_discriminator_match_that_fails_to_deserialize() {
    let valid = sample_trigger_order_filled();
    let resp = response_with(vec![
      corrupt_blob::<TriggerOrderFilledEvent>(),
      encode_event(&valid),
    ]);

    let got: TriggerOrderFilledEvent = parse_event_filtered(&resp)
      .expect("skips the corrupt match and returns the valid one");
    assert_eq!(got.nonce, 42);
  }
}
