use std::iter::once;

use anchor_client::solana_client::rpc_config::RpcSimulateTransactionConfig;
use anchor_client::solana_client::rpc_response::{
  Response, RpcSimulateTransactionResult,
};
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
use anyhow::{anyhow, Result};
use hylo_core::idl::tokens::{TokenMint, HYLOSOL, JITOSOL};
use itertools::Itertools;
use solana_transaction_status_client_types::{
  UiInstruction, UiParsedInstruction, UiPartiallyDecodedInstruction,
};

use crate::exchange_client::ExchangeClient;
use crate::prelude::VersionedTransactionData;
use crate::program_client::ProgramClient;
use crate::stability_pool_client::StabilityPoolClient;

pub trait LST: TokenMint {}
impl LST for JITOSOL {}
impl LST for HYLOSOL {}

pub const EXCHANGE_LOOKUP_TABLE: Pubkey =
  pubkey!("E1jD3vdypYukwy9SWgWCnAJEvKC4Uj7MEc3c4S2LogD9");

pub const STABILITY_POOL_LOOKUP_TABLE: Pubkey =
  pubkey!("Gb35n7SYMZCwCZbmxJMqoFsFX1mVhdSXmwo8wAJ8whWC");

pub const LST_REGISTRY_LOOKUP_TABLE: Pubkey =
  pubkey!("9Mb2Mt76AN7eNY3BBA4LgfTicARXhcEEokTBfsN47noK");

/// This wallet should hold at least one unit of jitoSOL, xSOL, hyUSD, and
/// sHYUSD. Useful for simulations of mint and redemption.
pub const REFERENCE_WALLET: Pubkey =
  pubkey!("GUX587fnbnZmqmq2hnav8r6siLczKS8wrp9QZRhuWeai");

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

/// Deserializes an account into an address lookup table.
///
/// # Errors
/// - Account data cannot be deserialized
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
/// - Failed to compile message
/// - Failed to create transaction
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
/// - Lookup table account doesn't exist
/// - Malformed structure (preamble cannot be split at 16)
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

/// Parses event type `E` from a simulated RPC call.
/// NB: Drops 16 bytes for header and discriminator.
///
/// # Errors
/// * No inner instructions found
/// * No parseable event found from target program
pub fn parse_event<E>(
  result: &Response<RpcSimulateTransactionResult>,
) -> Result<E>
where
  E: AnchorDeserialize + Discriminator,
{
  if let Some(ixs) = &result.value.inner_instructions {
    ixs
      .iter()
      .flat_map(|ix| ix.instructions.iter())
      .find_map(|ix| match ix {
        UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(
          UiPartiallyDecodedInstruction { data, .. },
        )) => bs58::decode(data)
          .into_vec()
          .ok()
          .filter(|decoded| &decoded[8..16] == E::DISCRIMINATOR)
          .and_then(|decoded| E::try_from_slice(&decoded[16..]).ok()),
        _ => None,
      })
      .ok_or(anyhow!("Parseable event not found"))
  } else {
    Err(anyhow!("Inner instructions not found"))
  }
}

/// Gets cluster from environment variables.
///
/// # Errors
/// - Missing `RPC_URL` or `RPC_WS_URL` environment variables
pub fn cluster_from_env() -> Result<Cluster> {
  let url = std::env::var("RPC_URL")?;
  let ws_url = std::env::var("RPC_WS_URL")?;
  Ok(Cluster::Custom(url, ws_url))
}

/// Builds test exchange client with random keypair.
///
/// # Errors
/// - Environment variable access
/// - Client initialization
pub fn build_test_exchange_client() -> Result<ExchangeClient> {
  let client = ExchangeClient::new_from_keypair(
    cluster_from_env()?,
    Keypair::new(),
    CommitmentConfig::confirmed(),
  )?;
  Ok(client)
}

/// Builds test stability pool client with random keypair.
///
/// # Errors
/// - Environment variable access
/// - Client initialization
pub fn build_test_stability_pool_client() -> Result<StabilityPoolClient> {
  let client = StabilityPoolClient::new_from_keypair(
    cluster_from_env()?,
    Keypair::new(),
    CommitmentConfig::confirmed(),
  )?;
  Ok(client)
}

/// Builds ATA creation instruction for a user and mint.
#[must_use]
pub fn user_ata_instruction(user: &Pubkey, mint: &Pubkey) -> Instruction {
  create_associated_token_account_idempotent(user, user, mint, &token::ID)
}
