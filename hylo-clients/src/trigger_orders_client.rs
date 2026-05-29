//! Off-chain client for `hylo-trigger-orders`. Permissionless — no admin
//! authority, no Squads wrapping.

use std::sync::Arc;

use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::Program;
use anyhow::Result;
use hylo_idl::exchange::accounts::{ExoPair, Hylo};
use hylo_idl::exchange::events::{
  ConvertLeverToStableExoEvent, ConvertLeverToStableLstEvent,
  ConvertStableToLeverExoEvent, ConvertStableToLeverLstEvent,
};
pub use hylo_idl::trigger_orders::accounts::TriggerOrder;
use hylo_idl::trigger_orders::client::args;
pub use hylo_idl::trigger_orders::constants::EXECUTOR_TIP_LAMPORTS;
pub use hylo_idl::trigger_orders::events::{
  TriggerOrderCancelled, TriggerOrderCreated, TriggerOrderFilled,
};
use hylo_idl::trigger_orders::instruction_builders;
pub use hylo_idl::trigger_orders::types::{
  ConvertDirection, PairTarget, TriggerDirection,
};
pub use hylo_idl::trigger_orders::{ExecutabilityBlocker, TriggerOutcome};
use hylo_idl::{pda, trigger_orders};

use crate::program_client::{ProgramClient, VersionedTransactionData};

/// Conservative CU limit for `execute_order_*` per on-chain spec §6.6.
/// Profile and tune after a first mainnet keeper run.
pub const CONSERVATIVE_EXECUTE_CU: u32 = 400_000;

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

  /// Permissionless: any signer can call. Fetches the current `Hylo`
  /// state to thread the SOL/USD oracle, builds the CPI, prepends 400k CU.
  ///
  /// # Errors
  /// * RPC error fetching `Hylo`
  /// * Account deserialization failure
  pub async fn execute_order_s2l_lst(
    &self,
    owner: Pubkey,
    nonce: u64,
  ) -> Result<VersionedTransactionData> {
    let executor = self.keypair.pubkey();
    let (order, _) =
      pda::trigger_order_lst(owner, ConvertDirection::StableToLever, nonce);
    let hylo: Hylo = self.program.account(pda::HYLO).await?;
    let ix = instruction_builders::execute_order_s2l_lst(
      executor, owner, order, &hylo,
    );
    Ok(
      VersionedTransactionData::one(ix)
        .with_compute_unit_limit(CONSERVATIVE_EXECUTE_CU),
    )
  }

  /// As `execute_order_s2l_lst`, but lever→stable.
  ///
  /// # Errors
  /// * RPC error fetching `Hylo`
  /// * Account deserialization failure
  pub async fn execute_order_l2s_lst(
    &self,
    owner: Pubkey,
    nonce: u64,
  ) -> Result<VersionedTransactionData> {
    let executor = self.keypair.pubkey();
    let (order, _) =
      pda::trigger_order_lst(owner, ConvertDirection::LeverToStable, nonce);
    let hylo: Hylo = self.program.account(pda::HYLO).await?;
    let ix = instruction_builders::execute_order_l2s_lst(
      executor, owner, order, &hylo,
    );
    Ok(
      VersionedTransactionData::one(ix)
        .with_compute_unit_limit(CONSERVATIVE_EXECUTE_CU),
    )
  }

  /// Stable→lever EXO. Fetches `Hylo` (for builder-signature uniformity)
  /// and the `ExoPair` (provides the collateral oracle).
  ///
  /// # Errors
  /// * RPC error fetching `Hylo`/`ExoPair`
  /// * Account deserialization failure
  pub async fn execute_order_s2l_exo(
    &self,
    owner: Pubkey,
    collateral_mint: Pubkey,
    nonce: u64,
  ) -> Result<VersionedTransactionData> {
    let executor = self.keypair.pubkey();
    let (order, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::StableToLever,
      collateral_mint,
      nonce,
    );
    // `Hylo` is fetched to satisfy the (uniform) execute-builder signature;
    // the EXO CPI takes its oracle from `ExoPair`, not `Hylo`.
    let hylo: Hylo = self.program.account(pda::HYLO).await?;
    let exo_pair: ExoPair =
      self.program.account(pda::exo_pair(collateral_mint)).await?;
    let ix = instruction_builders::execute_order_s2l_exo(
      executor,
      owner,
      order,
      collateral_mint,
      &hylo,
      &exo_pair,
    );
    Ok(
      VersionedTransactionData::one(ix)
        .with_compute_unit_limit(CONSERVATIVE_EXECUTE_CU),
    )
  }

  /// Lever→stable EXO. See `execute_order_s2l_exo` re: the `Hylo` fetch.
  ///
  /// # Errors
  /// * RPC error fetching `Hylo`/`ExoPair`
  /// * Account deserialization failure
  pub async fn execute_order_l2s_exo(
    &self,
    owner: Pubkey,
    collateral_mint: Pubkey,
    nonce: u64,
  ) -> Result<VersionedTransactionData> {
    let executor = self.keypair.pubkey();
    let (order, _) = pda::trigger_order_exo(
      owner,
      ConvertDirection::LeverToStable,
      collateral_mint,
      nonce,
    );
    let hylo: Hylo = self.program.account(pda::HYLO).await?;
    let exo_pair: ExoPair =
      self.program.account(pda::exo_pair(collateral_mint)).await?;
    let ix = instruction_builders::execute_order_l2s_exo(
      executor,
      owner,
      order,
      collateral_mint,
      &hylo,
      &exo_pair,
    );
    Ok(
      VersionedTransactionData::one(ix)
        .with_compute_unit_limit(CONSERVATIVE_EXECUTE_CU),
    )
  }

  // The `simulate_execute_order_*` methods below run TWO simulations (two RPC
  // round-trips) to extract the outer `TriggerOrderFilled` and the inner Hylo
  // `Convert*Event` separately. This is acceptable for v1 because `simulate_*`
  // is low-frequency (keeper pre-flight, not the hot path); it could be
  // optimized to a single simulation later (parse both events from one result
  // via `parse_event_filtered`, which is designed to be called repeatedly on
  // the same result).

  /// Simulate the execute path; return both events the chain emits on
  /// success: the trigger-orders `TriggerOrderFilled` AND the inner Hylo
  /// `ConvertStableToLeverLstEvent`.
  ///
  /// # Errors
  /// Simulation failure (CPI revert, account loading, CR-gate blockers);
  /// either event missing from the simulation result.
  pub async fn simulate_execute_order_s2l_lst(
    &self,
    owner: Pubkey,
    nonce: u64,
  ) -> Result<(TriggerOrderFilled, ConvertStableToLeverLstEvent)> {
    let vtd = self.execute_order_s2l_lst(owner, nonce).await?;
    let user = self.keypair.pubkey();
    let vt = self.build_simulation_transaction(&user, &vtd).await?;
    let outer: TriggerOrderFilled =
      self.simulate_transaction_event_filtered(&vt).await?;
    let inner: ConvertStableToLeverLstEvent =
      self.simulate_transaction_event_filtered(&vt).await?;
    Ok((outer, inner))
  }

  /// Simulate the execute path; return the outer `TriggerOrderFilled` and the
  /// inner Hylo `ConvertLeverToStableLstEvent`.
  ///
  /// # Errors
  /// Simulation failure (CPI revert, account loading, CR-gate blockers);
  /// either event missing from the simulation result.
  pub async fn simulate_execute_order_l2s_lst(
    &self,
    owner: Pubkey,
    nonce: u64,
  ) -> Result<(TriggerOrderFilled, ConvertLeverToStableLstEvent)> {
    let vtd = self.execute_order_l2s_lst(owner, nonce).await?;
    let user = self.keypair.pubkey();
    let vt = self.build_simulation_transaction(&user, &vtd).await?;
    let outer: TriggerOrderFilled =
      self.simulate_transaction_event_filtered(&vt).await?;
    let inner: ConvertLeverToStableLstEvent =
      self.simulate_transaction_event_filtered(&vt).await?;
    Ok((outer, inner))
  }

  /// Simulate the EXO execute path; return the outer `TriggerOrderFilled` and
  /// the inner Hylo `ConvertStableToLeverExoEvent`.
  ///
  /// # Errors
  /// Simulation failure (CPI revert, account loading, CR-gate blockers);
  /// either event missing from the simulation result.
  pub async fn simulate_execute_order_s2l_exo(
    &self,
    owner: Pubkey,
    collateral_mint: Pubkey,
    nonce: u64,
  ) -> Result<(TriggerOrderFilled, ConvertStableToLeverExoEvent)> {
    let vtd = self
      .execute_order_s2l_exo(owner, collateral_mint, nonce)
      .await?;
    let user = self.keypair.pubkey();
    let vt = self.build_simulation_transaction(&user, &vtd).await?;
    let outer: TriggerOrderFilled =
      self.simulate_transaction_event_filtered(&vt).await?;
    let inner: ConvertStableToLeverExoEvent =
      self.simulate_transaction_event_filtered(&vt).await?;
    Ok((outer, inner))
  }

  /// Simulate the EXO execute path; return the outer `TriggerOrderFilled` and
  /// the inner Hylo `ConvertLeverToStableExoEvent`.
  ///
  /// # Errors
  /// Simulation failure (CPI revert, account loading, CR-gate blockers);
  /// either event missing from the simulation result.
  pub async fn simulate_execute_order_l2s_exo(
    &self,
    owner: Pubkey,
    collateral_mint: Pubkey,
    nonce: u64,
  ) -> Result<(TriggerOrderFilled, ConvertLeverToStableExoEvent)> {
    let vtd = self
      .execute_order_l2s_exo(owner, collateral_mint, nonce)
      .await?;
    let user = self.keypair.pubkey();
    let vt = self.build_simulation_transaction(&user, &vtd).await?;
    let outer: TriggerOrderFilled =
      self.simulate_transaction_event_filtered(&vt).await?;
    let inner: ConvertLeverToStableExoEvent =
      self.simulate_transaction_event_filtered(&vt).await?;
    Ok((outer, inner))
  }

  /// Fetch a single open `TriggerOrder` by deriving its PDA. Returns
  /// `Ok(None)` if the PDA doesn't exist (e.g. already cancelled/executed).
  ///
  /// # Errors
  /// RPC error other than account-not-found.
  pub async fn get_order(
    &self,
    owner: Pubkey,
    direction: ConvertDirection,
    pair_target: PairTarget,
    nonce: u64,
  ) -> Result<Option<TriggerOrder>> {
    let (pda_pubkey, _) = match pair_target {
      PairTarget::Lst => pda::trigger_order_lst(owner, direction, nonce),
      PairTarget::Exo { collateral_mint } => {
        pda::trigger_order_exo(owner, direction, collateral_mint, nonce)
      }
    };
    match self.program.account::<TriggerOrder>(pda_pubkey).await {
      Ok(o) => Ok(Some(o)),
      Err(anchor_client::ClientError::AccountNotFound) => Ok(None),
      Err(e) => Err(e.into()),
    }
  }

  /// List all open orders for an owner via `getProgramAccounts` with a
  /// memcmp filter at `TriggerOrder::OWNER_OFFSET`.
  ///
  /// # Errors
  /// RPC error.
  pub async fn list_orders_by_owner(
    &self,
    owner: Pubkey,
  ) -> Result<Vec<(Pubkey, TriggerOrder)>> {
    use solana_rpc_client_api::filter::{Memcmp, RpcFilterType};
    let filters = vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
      TriggerOrder::OWNER_OFFSET,
      owner.to_bytes().to_vec(),
    ))];
    Ok(self.program.accounts::<TriggerOrder>(filters).await?)
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
  fn cu_constant_is_400k() {
    assert_eq!(CONSERVATIVE_EXECUTE_CU, 400_000);
  }

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
