use anchor_client::solana_client::rpc_config::RpcSimulateTransactionConfig;
use anchor_client::solana_client::rpc_response::{
  Response, RpcSimulateTransactionResult,
};
use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::address_lookup_table::state::AddressLookupTable;
use anchor_client::solana_sdk::address_lookup_table::AddressLookupTableAccount;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::{bs58, pubkey};
use anchor_lang::AnchorDeserialize;
use anyhow::{anyhow, Result};
use solana_transaction_status_client_types::{
  UiInstruction, UiParsedInstruction, UiPartiallyDecodedInstruction,
};

pub const EXCHANGE_LOOKUP_TABLE: Pubkey =
  pubkey!("E1jD3vdypYukwy9SWgWCnAJEvKC4Uj7MEc3c4S2LogD9");

pub const STABILITY_POOL_LOOKUP_TABLE: Pubkey =
  pubkey!("Gb35n7SYMZCwCZbmxJMqoFsFX1mVhdSXmwo8wAJ8whWC");

pub const LST_REGISTRY_LOOKUP_TABLE: Pubkey =
  pubkey!("9Mb2Mt76AN7eNY3BBA4LgfTicARXhcEEokTBfsN47noK");

/// Default configuration to use in simulated transactions.
#[must_use]
pub fn simulation_config() -> RpcSimulateTransactionConfig {
  RpcSimulateTransactionConfig {
    sig_verify: false,
    replace_recent_blockhash: true,
    commitment: Some(CommitmentConfig::confirmed()),
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

/// Parses event type `E` from a simulated RPC call.
///
/// # Errors
/// * No inner instructions found
/// * No parseable event found from target program
pub fn parse_event<E>(
  from_program_id: Pubkey,
  result: Response<RpcSimulateTransactionResult>,
) -> Result<E>
where
  E: AnchorDeserialize,
{
  if let Some(ixs) = result.value.inner_instructions {
    ixs
      .iter()
      .flat_map(|ix| ix.instructions.iter())
      .find_map(|ix| match ix {
        UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(
          UiPartiallyDecodedInstruction {
            program_id, data, ..
          },
        )) if *program_id == from_program_id.to_string() => bs58::decode(data)
          .into_vec()
          .ok()
          .and_then(|decoded| E::try_from_slice(&decoded).ok()),
        _ => None,
      })
      .ok_or(anyhow!("Parseable event not found"))
  } else {
    Err(anyhow!("Inner instructions not found"))
  }
}
