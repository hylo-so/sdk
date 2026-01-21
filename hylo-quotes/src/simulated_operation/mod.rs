//! Extract quote data from simulation events.

mod exchange;
mod stability_pool;

use anchor_lang::prelude::Pubkey;
use anchor_lang::{AnchorDeserialize, Discriminator};
use anyhow::Result;
use async_trait::async_trait;
use fix::typenum::Integer;
use hylo_clients::prelude::ProgramClient;
use hylo_clients::transaction::{BuildTransactionData, TransactionSyntax};
use hylo_idl::tokens::TokenMint;

use crate::token_operation::OperationOutput;
use crate::{ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER};

/// Compute unit details from simulation.
#[derive(Debug, Clone)]
pub struct ComputeUnitInfo {
  pub compute_units: u64,
  pub strategy: ComputeUnitStrategy,
}

/// Simulation counterpart to [`TokenOperation`]—extracts output from events
/// rather than computing from state.
///
/// [`TokenOperation`]: crate::token_operation::TokenOperation
pub trait SimulatedOperation<IN: TokenMint, OUT: TokenMint> {
  type FeeExp: Integer;
  type Event: AnchorDeserialize + Discriminator;

  /// # Errors
  /// * Event parsing or validation.
  fn quote_from_event(
    event: &Self::Event,
  ) -> Result<OperationOutput<IN::Exp, OUT::Exp, Self::FeeExp>>;
}

/// Turbofish helper for [`SimulatedOperation`].
#[async_trait]
pub trait SimulatedOperationExt {
  /// # Errors
  /// * Event parsing or validation.
  #[allow(clippy::type_complexity)]
  fn quote_from_event<IN: TokenMint, OUT: TokenMint>(
    event: &<Self as SimulatedOperation<IN, OUT>>::Event,
  ) -> Result<
    OperationOutput<
      IN::Exp,
      OUT::Exp,
      <Self as SimulatedOperation<IN, OUT>>::FeeExp,
    >,
  >
  where
    Self: SimulatedOperation<IN, OUT>;

  /// Simulates transaction and extracts output from event.
  ///
  /// # Errors
  /// * RPC simulation failure.
  /// * Event parsing or validation.
  #[allow(clippy::type_complexity)]
  async fn simulate_quote<IN: TokenMint, OUT: TokenMint>(
    &self,
    user: Pubkey,
    inputs: <Self as BuildTransactionData<IN, OUT>>::Inputs,
  ) -> Result<(
    OperationOutput<
      IN::Exp,
      OUT::Exp,
      <Self as SimulatedOperation<IN, OUT>>::FeeExp,
    >,
    ComputeUnitInfo,
  )>
  where
    Self: SimulatedOperation<IN, OUT>
      + BuildTransactionData<IN, OUT>
      + ProgramClient
      + TransactionSyntax
      + Send
      + Sync;
}

#[async_trait]
impl<X> SimulatedOperationExt for X {
  fn quote_from_event<IN: TokenMint, OUT: TokenMint>(
    event: &<Self as SimulatedOperation<IN, OUT>>::Event,
  ) -> Result<
    OperationOutput<
      IN::Exp,
      OUT::Exp,
      <Self as SimulatedOperation<IN, OUT>>::FeeExp,
    >,
  >
  where
    Self: SimulatedOperation<IN, OUT>,
  {
    <Self as SimulatedOperation<IN, OUT>>::quote_from_event(event)
  }

  async fn simulate_quote<IN: TokenMint, OUT: TokenMint>(
    &self,
    user: Pubkey,
    inputs: <Self as BuildTransactionData<IN, OUT>>::Inputs,
  ) -> Result<(
    OperationOutput<
      IN::Exp,
      OUT::Exp,
      <Self as SimulatedOperation<IN, OUT>>::FeeExp,
    >,
    ComputeUnitInfo,
  )>
  where
    Self: SimulatedOperation<IN, OUT>
      + BuildTransactionData<IN, OUT>
      + ProgramClient
      + TransactionSyntax
      + Send
      + Sync,
  {
    let (event, cus) = self
      .simulate_event_with_cus::<IN, OUT, <Self as SimulatedOperation<IN, OUT>>::Event>(
        user, inputs,
      )
      .await?;
    let output =
      <Self as SimulatedOperation<IN, OUT>>::quote_from_event(&event)?;
    let cu_info = match cus {
      Some(cu) if cu > 0 => ComputeUnitInfo {
        compute_units: cu,
        strategy: ComputeUnitStrategy::Simulated,
      },
      _ => ComputeUnitInfo {
        compute_units: DEFAULT_CUS_WITH_BUFFER,
        strategy: ComputeUnitStrategy::Estimated,
      },
    };
    Ok((output, cu_info))
  }
}
