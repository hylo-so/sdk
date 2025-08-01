use std::sync::Arc;

use anchor_client::solana_client::rpc_config::RpcSimulateTransactionConfig;
use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::address_lookup_table::state::AddressLookupTable;
use anchor_client::solana_sdk::address_lookup_table::AddressLookupTableAccount;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::message::{v0, VersionedMessage};
use anchor_client::solana_sdk::pubkey;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::solana_sdk::transaction::VersionedTransaction;
use anchor_client::{Client, Cluster, Program};
use anchor_lang::prelude::AccountMeta;
use anyhow::{anyhow, Result};
use itertools::Itertools;

use crate::hylo_exchange;

pub const EXCHANGE_LOOKUP_TABLE: Pubkey =
  pubkey!("E1jD3vdypYukwy9SWgWCnAJEvKC4Uj7MEc3c4S2LogD9");

pub const STABILITY_POOL_LOOKUP_TABLE: Pubkey =
  pubkey!("Gb35n7SYMZCwCZbmxJMqoFsFX1mVhdSXmwo8wAJ8whWC");

pub const LST_REGISTRY_LOOKUP_TABLE: Pubkey =
  pubkey!("9Mb2Mt76AN7eNY3BBA4LgfTicARXhcEEokTBfsN47noK");

pub const SOL_USD_PYTH_FEED: Pubkey =
  pubkey!("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");

pub const JITOSOL_MINT: Pubkey =
  pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");

/// IDL copies `UFixValue64` as a unique type. This converts to the canonical one from `fix`.
impl From<hylo_exchange::types::UFixValue64> for fix::prelude::UFixValue64 {
  fn from(val: hylo_exchange::types::UFixValue64) -> Self {
    fix::prelude::UFixValue64 {
      bits: val.bits,
      exp: val.exp,
    }
  }
}

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
fn deserialize_lookup_table(
  key: &Pubkey,
  account: &Account,
) -> Result<AddressLookupTableAccount> {
  let table = AddressLookupTable::deserialize(&account.data)?;
  Ok(AddressLookupTableAccount {
    key: *key,
    addresses: table.addresses.to_vec(),
  })
}

/// Abstracts the construction of client structs with `anchor_client::Program`.
#[async_trait::async_trait]
pub trait ProgramClient: Sized {
  const PROGRAM_ID: Pubkey;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> Self;

  fn program(&self) -> &Program<Arc<Keypair>>;

  fn keypair(&self) -> Arc<Keypair>;

  /// Constructs the given client with ID `Self::PROGRAM_ID`.
  ///
  /// # Errors
  /// - Underlying Anchor program creation
  fn new_from_keypair(
    cluster: Cluster,
    keypair: Keypair,
    config: CommitmentConfig,
  ) -> Result<Self> {
    let keypair = Arc::new(keypair);
    let client = Client::new_with_options(cluster, keypair.clone(), config);
    let program = client.program(Self::PROGRAM_ID)?;
    Ok(Self::build_client(program, keypair))
  }

  /// Builds a versioned transaction from instructions and lookup tables.
  ///
  /// # Errors
  /// - Failed to get the latest blockhash
  /// - Failed to compile the message
  /// - Failed to create the transaction
  /// - Failed to send the transaction
  async fn send_v0_transaction(
    &self,
    instructions: &[Instruction],
    lookup_tables: &[AddressLookupTableAccount],
  ) -> Result<Signature> {
    let recent_blockhash = self.program().rpc().get_latest_blockhash().await?;
    let message = v0::Message::try_compile(
      &self.program().payer(),
      instructions,
      lookup_tables,
      recent_blockhash,
    )?;
    let tx = VersionedTransaction::try_new(
      VersionedMessage::V0(message),
      &[self.keypair()],
    )?;
    let sig = self
      .program()
      .rpc()
      .send_and_confirm_transaction(&tx)
      .await?;
    Ok(sig)
  }

  /// Creates `remaining_accounts` array from LST registry table with all
  /// headers writable.
  ///
  /// # Errors
  /// - Lookup table account doesn't exist
  /// - Malformed structure (preamble cannot be split at 16)
  async fn load_lst_registry(
    &self,
  ) -> Result<(Vec<AccountMeta>, AddressLookupTableAccount)> {
    let table = self.load_lookup_table(&LST_REGISTRY_LOOKUP_TABLE).await?;
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

  /// Loads an address lookup table by public key.
  ///
  /// # Errors
  /// - Failed to fetch the account
  /// - Failed to deserialize account data
  async fn load_lookup_table(
    &self,
    key: &Pubkey,
  ) -> Result<AddressLookupTableAccount> {
    let account = self.program().rpc().get_account(key).await?;
    deserialize_lookup_table(key, &account)
  }

  /// Loads address lookup tables at given addresses.
  /// # Errors
  /// - Failed to fetch lookup table account
  /// - Failed to deserialize
  async fn load_multiple_lookup_tables(
    &self,
    pubkeys: &[Pubkey],
  ) -> Result<Vec<AddressLookupTableAccount>> {
    self
      .program()
      .rpc()
      .get_multiple_accounts(pubkeys)
      .await?
      .iter()
      .zip(pubkeys)
      .map(|(opt, key)| {
        if let Some(account) = opt {
          deserialize_lookup_table(key, account)
        } else {
          Err(anyhow!("No lookup table found at address {key}."))
        }
      })
      .try_collect()
  }
}
