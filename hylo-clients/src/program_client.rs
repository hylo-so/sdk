use std::iter::once;
use std::sync::Arc;

use anchor_client::solana_sdk::address_lookup_table::AddressLookupTableAccount;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::message::{v0, VersionedMessage};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::solana_sdk::transaction::VersionedTransaction;
use anchor_client::{Client, Cluster, Program};
use anchor_lang::prelude::AccountMeta;
use anchor_lang::{AnchorDeserialize, Discriminator};
use anyhow::{anyhow, Result};
use base64::prelude::{Engine, BASE64_STANDARD};
use itertools::Itertools;

use crate::util::{
  deserialize_lookup_table, parse_event, simulation_config,
  LST_REGISTRY_LOOKUP_TABLE,
};

/// Components from which a [`VersionedTransaction`] can be built.
pub struct VersionedTransactionData {
  pub instructions: Vec<Instruction>,
  pub lookup_tables: Vec<AddressLookupTableAccount>,
}

impl VersionedTransactionData {
  #[must_use]
  pub fn no_lookup(instructions: Vec<Instruction>) -> VersionedTransactionData {
    VersionedTransactionData {
      instructions,
      lookup_tables: vec![],
    }
  }

  #[must_use]
  pub fn new(
    instructions: Vec<Instruction>,
    lookup_tables: Vec<AddressLookupTableAccount>,
  ) -> VersionedTransactionData {
    VersionedTransactionData {
      instructions,
      lookup_tables,
    }
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

  /// Constructs the program client with a given keypair and associated program
  /// ID.
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

  /// Constructs the program client with a random keypair.
  ///
  /// # Errors
  /// - Underlying Anchor program creation
  fn new_random_keypair(
    cluster: Cluster,
    config: CommitmentConfig,
  ) -> Result<Self> {
    let keypair = Keypair::new();
    Self::new_from_keypair(cluster, keypair, config)
  }

  /// Builds a versioned transaction from instructions and lookup tables.
  ///
  /// # Errors
  /// - Failed to get latest blockhash
  /// - Failed to compile message
  /// - Failed to create transaction
  async fn build_v0_transaction(
    &self,
    VersionedTransactionData {
      instructions,
      lookup_tables,
    }: &VersionedTransactionData,
  ) -> Result<VersionedTransaction> {
    let recent_blockhash = self.program().rpc().get_latest_blockhash().await?;
    let message = v0::Message::try_compile(
      &self.keypair().pubkey(),
      instructions,
      lookup_tables,
      recent_blockhash,
    )?;
    let signatures = vec![self.keypair().sign_message(&message.serialize())];
    let tx = VersionedTransaction {
      message: VersionedMessage::V0(message),
      signatures,
    };
    Ok(tx)
  }

  /// Builds a versioned transaction with additional signers.
  ///
  /// # Errors
  /// - Failed to get latest blockhash
  /// - Failed to compile message
  /// - Failed to create transaction
  async fn build_v0_transaction_extra_signers(
    &self,
    VersionedTransactionData {
      instructions,
      lookup_tables,
    }: &VersionedTransactionData,
    additional_signers: Vec<Keypair>,
  ) -> Result<VersionedTransaction> {
    let recent_blockhash = self.program().rpc().get_latest_blockhash().await?;
    let message = v0::Message::try_compile(
      &self.keypair().pubkey(),
      instructions,
      lookup_tables,
      recent_blockhash,
    )?;
    let signatures = once(self.keypair().as_ref())
      .chain(additional_signers.iter())
      .map(|signer| signer.sign_message(&message.serialize()))
      .collect_vec();
    let tx = VersionedTransaction {
      message: VersionedMessage::V0(message),
      signatures,
    };
    Ok(tx)
  }

  /// Builds versioned transaction with dummy signatures for simulation.
  ///
  /// # Errors
  /// - Failed to get latest blockhash
  /// - Failed to compile message
  /// - Failed to create transaction
  async fn build_simulation_transaction(
    &self,
    for_user: &Pubkey,
    VersionedTransactionData {
      instructions,
      lookup_tables,
    }: &VersionedTransactionData,
  ) -> Result<VersionedTransaction> {
    let recent_blockhash = self.program().rpc().get_latest_blockhash().await?;
    let message = v0::Message::try_compile(
      for_user,
      instructions,
      lookup_tables,
      recent_blockhash,
    )?;
    let num_sigs = message.header.num_required_signatures.into();
    let dummy_signatures = vec![Signature::default(); num_sigs];
    let tx = VersionedTransaction {
      message: VersionedMessage::V0(message),
      signatures: dummy_signatures,
    };
    Ok(tx)
  }

  /// Sends a versioned transaction from instructions and lookup tables.
  ///
  /// # Errors
  /// - Failed to build transaction
  /// - Failed to send and confirm transaction
  async fn send_v0_transaction(
    &self,
    args: &VersionedTransactionData,
  ) -> Result<Signature> {
    let tx = self.build_v0_transaction(args).await?;
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

  /// Simulates transaction and returns deserialized return data.
  ///
  /// # Errors
  /// * Transaction simulation fails
  /// * No return data found in simulation result
  /// * Base64 decoding of return data fails
  /// * Deserialization of return data fails
  async fn simulate_transaction_return<R: AnchorDeserialize>(
    &self,
    tx: VersionedTransaction,
  ) -> Result<R> {
    let rpc = self.program().rpc();
    let result = rpc
      .simulate_transaction_with_config(&tx, simulation_config())
      .await?;
    let (data, _) = result
      .value
      .return_data
      .ok_or(anyhow!("Return data not found"))?
      .data;
    let bytes = BASE64_STANDARD.decode(data)?;
    let ret = R::try_from_slice(&bytes)?;
    Ok(ret)
  }

  /// Simulates transaction and extracts event from CPI instructions.
  ///
  /// # Errors
  /// * Transaction simulation fails
  /// * Event parsing from CPI instructions fails
  /// * Event deserialization fails
  async fn simulate_transaction_event<E: AnchorDeserialize + Discriminator>(
    &self,
    tx: &VersionedTransaction,
  ) -> Result<E> {
    let rpc = self.program().rpc();
    let result = rpc
      .simulate_transaction_with_config(tx, simulation_config())
      .await?;
    parse_event(&result)
  }
}
