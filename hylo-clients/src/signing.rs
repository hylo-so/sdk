use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use squads_multisig::pda::get_vault_pda;

use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::squads::{SquadsContext, SquadsTransactionData};

/// Selects how an administrative instruction is authorized and submitted.
#[derive(Clone, Copy, Debug, Default)]
pub enum SigningMethod {
  /// The client's payer signs and submits the instruction directly.
  #[default]
  Direct,
  /// A Squads vault signs the inner instruction after proposal approval.
  Squads { multisig: Pubkey, vault_index: u8 },
}

/// A completed administrative transaction ready for CLI-specific handling.
pub enum SignedTransaction {
  /// A normal transaction signed by the client's payer.
  Direct(VersionedTransactionData),
  /// A Squads vault transaction and proposal signed by the client's payer.
  Squads(SquadsTransactionData),
}

impl SigningMethod {
  /// Returns the authority account expected by the inner instruction.
  #[must_use]
  pub fn authority(self, payer: Pubkey) -> Pubkey {
    match self {
      Self::Direct => payer,
      Self::Squads {
        multisig,
        vault_index,
      } => get_vault_pda(&multisig, vault_index, None).0,
    }
  }

  /// Finalizes an administrative transaction using the configured method.
  ///
  /// # Errors
  /// * Failed to fetch the next Squads transaction index.
  /// * Failed to compile a Squads vault transaction message.
  pub async fn prepare(
    self,
    rpc: &RpcClient,
    creator: Pubkey,
    inner: VersionedTransactionData,
    memo: String,
  ) -> Result<SignedTransaction> {
    match self {
      Self::Direct => Ok(SignedTransaction::Direct(inner)),
      Self::Squads {
        multisig,
        vault_index,
      } => {
        let context = SquadsContext::new(rpc, multisig, vault_index).await?;
        Ok(SignedTransaction::Squads(
          context.build_proposal(&inner, creator, memo)?,
        ))
      }
    }
  }
}

#[async_trait::async_trait]
pub trait SigningClient: ProgramClient {
  fn signing_method(&self) -> SigningMethod;

  #[must_use]
  fn with_signing_method(self, signing_method: SigningMethod) -> Self
  where
    Self: Sized;

  fn signer(&self) -> Pubkey {
    self.signing_method().authority(self.program().payer())
  }

  async fn sign(
    &self,
    inner: VersionedTransactionData,
    memo: String,
  ) -> Result<SignedTransaction> {
    self
      .signing_method()
      .prepare(&self.program().rpc(), self.program().payer(), inner, memo)
      .await
  }
}

#[cfg(test)]
mod tests {
  use anchor_client::solana_sdk::instruction::Instruction;
  use anchor_client::solana_sdk::pubkey::Pubkey;
  use anyhow::Result;
  use squads_multisig::pda::get_vault_pda;

  use super::SigningMethod;
  use crate::program_client::VersionedTransactionData;
  use crate::squads::SquadsContext;

  #[test]
  fn direct_uses_the_client_payer_as_authority() {
    let payer = Pubkey::new_unique();
    assert_eq!(SigningMethod::Direct.authority(payer), payer);
  }

  #[test]
  fn squads_uses_its_vault_pda_as_authority() {
    let multisig = Pubkey::new_unique();
    let method = SigningMethod::Squads {
      multisig,
      vault_index: 3,
    };
    assert_eq!(
      method.authority(Pubkey::new_unique()),
      get_vault_pda(&multisig, 3, None).0
    );
  }

  #[test]
  fn squads_proposal_wraps_the_inner_transaction() -> Result<()> {
    let multisig = Pubkey::new_unique();
    let context = SquadsContext {
      multisig,
      vault_index: 0,
      transaction_index: 1,
    };
    let data = context.build_proposal(
      &VersionedTransactionData::one(Instruction::new_with_bytes(
        Pubkey::new_unique(),
        &[],
        vec![],
      )),
      Pubkey::new_unique(),
      "memo".to_owned(),
    )?;
    assert_eq!(data.memo, "memo");
    assert_eq!(data.transaction.instructions.len(), 2);
    Ok(())
  }
}
