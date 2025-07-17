use std::sync::Arc;

use anchor_client::solana_client::rpc_config::RpcSimulateTransactionConfig;
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
use anyhow::Result;
use futures::future::try_join_all;

pub const EXCHANGE_LOOKUP_TABLE: Pubkey =
  pubkey!("E1jD3vdypYukwy9SWgWCnAJEvKC4Uj7MEc3c4S2LogD9");

pub const LST_REGISTRY_LOOKUP_TABLE: Pubkey =
  pubkey!("9Mb2Mt76AN7eNY3BBA4LgfTicARXhcEEokTBfsN47noK");

pub const SOL_USD_PYTH_FEED: Pubkey =
  pubkey!("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");

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

  /// Loads address lookup tables at given addresses.
  ///
  /// # Errors
  /// - Failed to fetch lookup table account
  /// - Failed to deserialize
  async fn load_lookup_tables(
    &self,
    pubkeys: &[Pubkey],
  ) -> Result<Vec<AddressLookupTableAccount>> {
    let futures = pubkeys.iter().map(|key| async {
      let account = self.program().rpc().get_account(key).await?;
      let lut = AddressLookupTable::deserialize(&account.data)?;
      Ok(AddressLookupTableAccount {
        key: *key,
        addresses: lut.addresses.to_vec(),
      })
    });
    try_join_all(futures).await
  }
}
