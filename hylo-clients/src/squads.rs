//! Squads v4 integration for wrapping protocol instructions in a
//! multisig vault transaction.

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_lang::system_program;
use anyhow::Result;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use squads_multisig::client::{
  get_multisig, proposal_create, vault_transaction_create,
  ProposalCreateAccounts, ProposalCreateArgs, VaultTransactionCreateAccounts,
};
use squads_multisig::pda::{
  get_proposal_pda, get_transaction_pda, get_vault_pda,
};
use squads_multisig::squads_multisig_program::TransactionMessage;
use squads_multisig::vault_transaction::VaultTransactionMessageExt;

use crate::program_client::VersionedTransactionData;

/// Squads multisig to which a vault transaction can be sent.
#[derive(Debug, Clone, Copy)]
pub struct SquadsContext {
  pub multisig: Pubkey,
  pub vault_index: u8,
  pub transaction_index: u64,
}

impl SquadsContext {
  /// Constructs a context targeting the next free `transaction_index`
  /// on the given multisig.
  ///
  /// # Errors
  /// * Failed to fetch or deserialize the multisig account
  pub async fn new(
    rpc: &RpcClient,
    multisig: Pubkey,
    vault_index: u8,
  ) -> Result<SquadsContext> {
    let account = get_multisig(rpc, &multisig).await?;
    Ok(SquadsContext {
      multisig,
      vault_index,
      transaction_index: account.transaction_index + 1,
    })
  }

  #[must_use]
  pub fn vault_pda(&self) -> Pubkey {
    get_vault_pda(&self.multisig, self.vault_index, None).0
  }

  #[must_use]
  pub fn transaction_pda(&self) -> Pubkey {
    get_transaction_pda(&self.multisig, self.transaction_index, None).0
  }

  #[must_use]
  pub fn proposal_pda(&self) -> Pubkey {
    get_proposal_pda(&self.multisig, self.transaction_index, None).0
  }

  fn create_vault(
    &self,
    creator: Pubkey,
    message: &TransactionMessage,
    memo: String,
  ) -> Instruction {
    const NO_EPHEMERAL_SIGNERS: u8 = 0;
    let accounts = VaultTransactionCreateAccounts {
      multisig: self.multisig,
      transaction: self.transaction_pda(),
      creator,
      rent_payer: creator,
      system_program: system_program::ID,
    };
    vault_transaction_create(
      accounts,
      self.vault_index,
      NO_EPHEMERAL_SIGNERS,
      message,
      Some(memo),
      None,
    )
  }

  fn create_proposal(&self, creator: Pubkey) -> Instruction {
    let accounts = ProposalCreateAccounts {
      multisig: self.multisig,
      proposal: self.proposal_pda(),
      creator,
      rent_payer: creator,
      system_program: system_program::ID,
    };
    let args = ProposalCreateArgs {
      transaction_index: self.transaction_index,
      draft: false,
    };
    proposal_create(accounts, args, None)
  }

/// Wraps `inner` instructions into vault execution and proposal transaction.
  /// Inner instructions must use [`Self::vault_pda`] as the admin signer.
  ///
  /// # Errors
  /// * Failed to compile the inner message
  pub fn wrap(
    &self,
    inner: &VersionedTransactionData,
    creator: Pubkey,
    memo: String,
  ) -> Result<VersionedTransactionData> {
    let message = TransactionMessage::try_compile(
      &self.vault_pda(),
      &inner.instructions,
      &inner.lookup_tables,
    )?;
    let vault_ix = self.create_vault(creator, &message, memo);
    let proposal_ix = self.create_proposal(creator);
    Ok(VersionedTransactionData::new(
      vec![vault_ix, proposal_ix],
      vec![],
    ))
  }
}
