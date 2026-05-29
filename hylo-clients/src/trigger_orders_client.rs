//! Off-chain client for `hylo-trigger-orders`. Permissionless — no admin
//! authority, no Squads wrapping.

use std::sync::Arc;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::Program;
use anyhow::Result;
use hylo_idl::trigger_orders::client::args;
use hylo_idl::trigger_orders::instruction_builders;
use hylo_idl::trigger_orders::types::{
  ConvertDirection, PairTarget, TriggerDirection,
};
use hylo_idl::{pda, trigger_orders};

use crate::program_client::{ProgramClient, VersionedTransactionData};

/// Permissionless client for the Hylo trigger-orders program. Manages
/// trigger-order placement, cancellation, and execution. Unlike the admin
/// clients, there is no authority or Squads wrapping — any keypair can drive
/// these operations.
pub struct TriggerOrdersClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for TriggerOrdersClient {
  const PROGRAM_ID: Pubkey = trigger_orders::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> TriggerOrdersClient {
    TriggerOrdersClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl TriggerOrdersClient {
  /// Create a stable→lever LST trigger order. Escrows HYUSD from the
  /// signer's ATA plus an `EXECUTOR_TIP_LAMPORTS` lamport tip.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  // Builds no RPC round-trip today, but keeps an `async` signature for a
  // uniform `TriggerOrdersClient` surface (matching cancel/execute, some of
  // which do await) and so future escrow/balance lookups can be added without
  // a breaking API change.
  #[allow(clippy::unused_async)]
  pub async fn create_order_s2l_lst(
    &self,
    nonce: u64,
    escrow_amount: u64,
    trigger_price: i64,
    trigger_expo: i32,
    direction: TriggerDirection,
  ) -> Result<VersionedTransactionData> {
    let owner = self.keypair.pubkey();
    let (order, _) =
      pda::trigger_order_lst(owner, ConvertDirection::StableToLever, nonce);
    let ix = instruction_builders::create_order_s2l_lst(
      owner,
      order,
      &args::CreateOrderS2lLst {
        nonce,
        escrow_amount,
        trigger_price,
        trigger_expo,
        direction,
      },
    );
    Ok(VersionedTransactionData::one(ix))
  }

  /// As `create_order_s2l_lst`, but for an EXO pair.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  #[allow(clippy::unused_async)] // async for surface uniformity; see s2l_lst.
  pub async fn create_order_s2l_exo(
    &self,
    collateral_mint: Pubkey,
    nonce: u64,
    escrow_amount: u64,
    trigger_price: i64,
    trigger_expo: i32,
    direction: TriggerDirection,
  ) -> Result<VersionedTransactionData> {
    let owner = self.keypair.pubkey();
    let (order, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::StableToLever,
      collateral_mint,
      nonce,
    );
    let ix = instruction_builders::create_order_s2l_exo(
      owner,
      order,
      collateral_mint,
      &args::CreateOrderS2lExo {
        nonce,
        escrow_amount,
        trigger_price,
        trigger_expo,
        direction,
      },
    );
    Ok(VersionedTransactionData::one(ix))
  }

  /// Lever→Stable LST. Escrows xSOL.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  #[allow(clippy::unused_async)] // async for surface uniformity; see s2l_lst.
  pub async fn create_order_l2s_lst(
    &self,
    nonce: u64,
    escrow_amount: u64,
    trigger_price: i64,
    trigger_expo: i32,
    direction: TriggerDirection,
  ) -> Result<VersionedTransactionData> {
    let owner = self.keypair.pubkey();
    let (order, _) =
      pda::trigger_order_lst(owner, ConvertDirection::LeverToStable, nonce);
    let ix = instruction_builders::create_order_l2s_lst(
      owner,
      order,
      &args::CreateOrderL2sLst {
        nonce,
        escrow_amount,
        trigger_price,
        trigger_expo,
        direction,
      },
    );
    Ok(VersionedTransactionData::one(ix))
  }

  /// Lever→Stable EXO. Escrows the per-collateral levercoin.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  #[allow(clippy::unused_async)] // async for surface uniformity; see s2l_lst.
  pub async fn create_order_l2s_exo(
    &self,
    collateral_mint: Pubkey,
    nonce: u64,
    escrow_amount: u64,
    trigger_price: i64,
    trigger_expo: i32,
    direction: TriggerDirection,
  ) -> Result<VersionedTransactionData> {
    let owner = self.keypair.pubkey();
    let (order, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::LeverToStable,
      collateral_mint,
      nonce,
    );
    let ix = instruction_builders::create_order_l2s_exo(
      owner,
      order,
      collateral_mint,
      &args::CreateOrderL2sExo {
        nonce,
        escrow_amount,
        trigger_price,
        trigger_expo,
        direction,
      },
    );
    Ok(VersionedTransactionData::one(ix))
  }

  /// Cancel an s2l trigger order. Refunds HYUSD escrow + executor tip to
  /// the owner via Anchor's `close = owner`. Handles both LST and EXO s2l
  /// (the s2l escrow is always HYUSD), selected by `pair_target`.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  #[allow(clippy::unused_async)] // async for surface uniformity; see s2l_lst.
  pub async fn cancel_order_s2l(
    &self,
    nonce: u64,
    pair_target: PairTarget,
  ) -> Result<VersionedTransactionData> {
    let owner = self.keypair.pubkey();
    let (order, _) = match pair_target {
      PairTarget::Lst => {
        pda::trigger_order_lst(owner, ConvertDirection::StableToLever, nonce)
      }
      PairTarget::Exo { collateral_mint } => pda::trigger_order_exo(
        owner,
        ConvertDirection::StableToLever,
        collateral_mint,
        nonce,
      ),
    };
    let ix = instruction_builders::cancel_order_s2l(
      owner,
      order,
      &args::CancelOrderS2l {},
    );
    Ok(VersionedTransactionData::one(ix))
  }

  /// Cancel an l2s LST order. Refunds xSOL escrow + tip via `close = owner`.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  #[allow(clippy::unused_async)] // async for surface uniformity; see s2l_lst.
  pub async fn cancel_order_l2s_lst(
    &self,
    nonce: u64,
  ) -> Result<VersionedTransactionData> {
    let owner = self.keypair.pubkey();
    let (order, _) =
      pda::trigger_order_lst(owner, ConvertDirection::LeverToStable, nonce);
    let ix = instruction_builders::cancel_order_l2s_lst(
      owner,
      order,
      &args::CancelOrderL2sLst {},
    );
    Ok(VersionedTransactionData::one(ix))
  }

  /// Cancel an l2s EXO order. Refunds the per-collateral levercoin escrow +
  /// tip.
  ///
  /// # Errors
  /// * Failed to build transaction instructions
  #[allow(clippy::unused_async)] // async for surface uniformity; see s2l_lst.
  pub async fn cancel_order_l2s_exo(
    &self,
    collateral_mint: Pubkey,
    nonce: u64,
  ) -> Result<VersionedTransactionData> {
    let owner = self.keypair.pubkey();
    let (order, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::LeverToStable,
      collateral_mint,
      nonce,
    );
    let ix = instruction_builders::cancel_order_l2s_exo(
      owner,
      order,
      collateral_mint,
      &args::CancelOrderL2sExo {},
    );
    Ok(VersionedTransactionData::one(ix))
  }
}

#[cfg(test)]
mod tests {
  use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
  use anchor_client::Cluster;
  use hylo_idl::pda;
  use hylo_idl::trigger_orders::types::{
    ConvertDirection, PairTarget, TriggerDirection,
  };

  use super::*;

  #[test]
  fn client_id_matches_trigger_orders_program_id() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .expect("build");
    assert_eq!(
      <TriggerOrdersClient as ProgramClient>::PROGRAM_ID,
      trigger_orders::ID
    );
    let _ = c.keypair(); // exercise the trait method
  }

  #[tokio::test]
  async fn create_order_s2l_lst_routes_to_correct_pda() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .unwrap();
    let owner = c.keypair().pubkey();

    let tx = c
      .create_order_s2l_lst(7, 1000, 100, -8, TriggerDirection::AtOrAbove)
      .await
      .expect("build tx");

    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, trigger_orders::ID);

    let (expected_pda, _) =
      pda::trigger_order_lst(owner, ConvertDirection::StableToLever, 7);
    // The `order` account is at position 1 (owner, then order).
    assert_eq!(tx.instructions[0].accounts[1].pubkey, expected_pda);
  }

  #[tokio::test]
  async fn create_order_s2l_exo_derives_with_collateral_mint() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .unwrap();
    let owner = c.keypair().pubkey();
    let collateral_mint = Pubkey::new_unique();

    let tx = c
      .create_order_s2l_exo(
        collateral_mint,
        3,
        500,
        100,
        -8,
        TriggerDirection::AtOrAbove,
      )
      .await
      .expect("build tx");

    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, trigger_orders::ID);

    let (expected_pda, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::StableToLever,
      collateral_mint,
      3,
    );
    assert_eq!(tx.instructions[0].accounts[1].pubkey, expected_pda);
  }

  #[tokio::test]
  async fn create_order_l2s_lst_routes_to_correct_pda() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .unwrap();
    let owner = c.keypair().pubkey();

    let tx = c
      .create_order_l2s_lst(11, 750, 25, -8, TriggerDirection::AtOrBelow)
      .await
      .expect("build tx");

    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, trigger_orders::ID);

    let (expected_pda, _) =
      pda::trigger_order_lst(owner, ConvertDirection::LeverToStable, 11);
    assert_eq!(tx.instructions[0].accounts[1].pubkey, expected_pda);
  }

  #[tokio::test]
  async fn create_order_l2s_exo_derives_with_collateral_mint() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .unwrap();
    let owner = c.keypair().pubkey();
    let collateral_mint = Pubkey::new_unique();
    let tx = c
      .create_order_l2s_exo(
        collateral_mint,
        9,
        2_000,
        50,
        -8,
        TriggerDirection::AtOrBelow,
      )
      .await
      .expect("build tx");

    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, trigger_orders::ID);

    let (expected_pda, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::LeverToStable,
      collateral_mint,
      9,
    );
    assert_eq!(tx.instructions[0].accounts[1].pubkey, expected_pda);
  }

  #[tokio::test]
  async fn cancel_order_s2l_lst_derives_lst_pda() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .unwrap();
    let owner = c.keypair().pubkey();

    let tx = c
      .cancel_order_s2l(11, PairTarget::Lst)
      .await
      .expect("build tx");

    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, trigger_orders::ID);

    let (expected_pda, _) =
      pda::trigger_order_lst(owner, ConvertDirection::StableToLever, 11);
    // The `order` account is at position 1 (owner, then order).
    assert_eq!(tx.instructions[0].accounts[1].pubkey, expected_pda);
  }

  #[tokio::test]
  async fn cancel_order_s2l_exo_derives_exo_pda() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .unwrap();
    let owner = c.keypair().pubkey();
    let collateral_mint = Pubkey::new_unique();

    let tx = c
      .cancel_order_s2l(12, PairTarget::Exo { collateral_mint })
      .await
      .expect("build tx");

    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, trigger_orders::ID);

    let (expected_pda, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::StableToLever,
      collateral_mint,
      12,
    );
    assert_eq!(tx.instructions[0].accounts[1].pubkey, expected_pda);
  }

  #[tokio::test]
  async fn cancel_order_l2s_lst_derives_pda() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .unwrap();
    let owner = c.keypair().pubkey();

    let tx = c.cancel_order_l2s_lst(13).await.expect("build tx");

    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, trigger_orders::ID);

    let (expected_pda, _) =
      pda::trigger_order_lst(owner, ConvertDirection::LeverToStable, 13);
    assert_eq!(tx.instructions[0].accounts[1].pubkey, expected_pda);
  }

  #[tokio::test]
  async fn cancel_order_l2s_exo_derives_pda() {
    let c = TriggerOrdersClient::new_random_keypair(
      Cluster::Localnet,
      CommitmentConfig::confirmed(),
    )
    .unwrap();
    let owner = c.keypair().pubkey();
    let collateral_mint = Pubkey::new_unique();

    let tx = c
      .cancel_order_l2s_exo(collateral_mint, 14)
      .await
      .expect("build tx");

    assert_eq!(tx.instructions.len(), 1);
    assert_eq!(tx.instructions[0].program_id, trigger_orders::ID);

    let (expected_pda, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::LeverToStable,
      collateral_mint,
      14,
    );
    assert_eq!(tx.instructions[0].accounts[1].pubkey, expected_pda);
  }
}
