#![allow(clippy::upper_case_acronyms)]

use anchor_client::solana_sdk::signature::Signature;
use anchor_lang::prelude::Pubkey;
use anchor_lang::{AnchorDeserialize, Discriminator};
use anyhow::Result;
use fix::prelude::*;
use hylo_core::slippage_config::SlippageConfig;

use crate::program_client::{ProgramClient, VersionedTransactionData};

/// Simulates one unit of token pair exchange via RPC simulation against
/// protocol.
///
/// # Type Parameters
/// - `I`: Input token (e.g., `JITOSOL`, `HYUSD`, `XSOL`, `SHYUSD`)
/// - `O`: Output token
///
/// # Associated Types
/// - `OutExp`: Fixed point precision exponent for the output amount (e.g. `N6`
///   for `UFix64<N6>`)
/// - `Event`: IDL event type emitted by the simulated transaction
#[async_trait::async_trait]
pub trait SimulatePrice<I, O>: BuildTransactionData<I, O> + ProgramClient {
  type OutExp;
  type Event: AnchorDeserialize + Discriminator;

  /// Extracts the output amount from a simulation event.
  ///
  /// # Errors
  /// Event parsing or conversion errors
  fn from_event(e: &Self::Event) -> Result<UFix64<Self::OutExp>>;

  /// Simulates transaction with actual inputs and returns the full event.
  ///
  /// This allows callers to extract both output amounts and fees from the
  /// event, rather than just the output amount via `from_event`.
  async fn simulate_event(
    &self,
    user: Pubkey,
    inputs: Self::Inputs,
  ) -> Result<Self::Event> {
    let args = self.build(inputs).await?;
    let tx = self.build_simulation_transaction(&user, &args).await?;
    self.simulate_transaction_event::<Self::Event>(&tx).await
  }

  /// Simulates transaction and returns the event and compute units consumed.
  ///
  /// Returns `(event, compute_units)` where:
  /// - `event`: The transaction event containing amounts and fees
  /// - `compute_units`: `Some(u64)` if available from simulation, `None`
  ///   otherwise
  async fn simulate_event_with_cus(
    &self,
    user: Pubkey,
    inputs: Self::Inputs,
  ) -> Result<(Self::Event, Option<u64>)> {
    let args = self.build(inputs).await?;
    let tx = self.build_simulation_transaction(&user, &args).await?;
    self
      .simulate_transaction_event_with_cus::<Self::Event>(&tx)
      .await
  }
}

/// Arguments for minting operations that deposit LST to mint hyUSD or xSOL.
pub struct MintArgs {
  pub amount: UFix64<N9>,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

/// Arguments for redemption operations that burn hyUSD or xSOL to withdraw LST.
pub struct RedeemArgs {
  pub amount: UFix64<N6>,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

/// Arguments for swap operations between hyUSD and xSOL.
pub struct SwapArgs {
  pub amount: UFix64<N6>,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

/// Arguments for swap operations between LSTs held in exchange.
pub struct LstSwapArgs {
  pub amount_lst_a: UFix64<N9>,
  pub lst_a_mint: Pubkey,
  pub lst_b_mint: Pubkey,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

/// Arguments for stability pool operations (deposit/withdraw sHYUSD).
pub struct StabilityPoolArgs {
  pub amount: UFix64<N6>,
  pub user: Pubkey,
}

/// Builds transaction data (instructions and lookup tables) for operations.
///
/// # Type Parameters
/// - `I`: Input token
/// - `O`: Output token
///
/// # Associated Types
/// - `Inputs`: Parameter type for building transactions (e.g., `MintArgs`,
///   `SwapArgs`)
#[async_trait::async_trait]
pub trait BuildTransactionData<I, O> {
  type Inputs: Send + Sync + 'static;

  /// Builds versioned transaction data for the token pair operation.
  async fn build(
    &self,
    inputs: Self::Inputs,
  ) -> Result<VersionedTransactionData>;
}

/// High-level API for transaction operations.
#[async_trait::async_trait]
pub trait TransactionSyntax {
  /// Executes transaction by building and sending it.
  async fn run_transaction<I, O>(
    &self,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<Signature>
  where
    Self: BuildTransactionData<I, O> + ProgramClient,
  {
    let args = self.build(inputs).await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Builds transaction data without executing.
  async fn build_transaction_data<I, O>(
    &self,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<VersionedTransactionData>
  where
    Self: BuildTransactionData<I, O>,
  {
    self.build(inputs).await
  }
}
