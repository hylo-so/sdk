//! Extract quote data from simulation events.

mod exchange;
mod stability_pool;

use anchor_lang::{AnchorDeserialize, Discriminator};
use anyhow::Result;
use fix::typenum::Integer;
use hylo_idl::tokens::TokenMint;

use crate::token_operation::OperationOutput;

/// Simulation counterpart to [`TokenOperation`]—extracts output from events
/// rather than computing from state.
///
/// [`TokenOperation`]: crate::token_operation::TokenOperation
pub trait SimulatedOperation<IN: TokenMint, OUT: TokenMint> {
  type FeeExp: Integer;
  type Event: AnchorDeserialize + Discriminator;

  /// # Errors
  /// * Event parsing or validation.
  fn from_event(
    event: &Self::Event,
  ) -> Result<OperationOutput<IN::Exp, OUT::Exp, Self::FeeExp>>;
}

/// Turbofish helper for [`SimulatedOperation`].
pub trait SimulatedOperationExt {
  /// # Errors
  /// * Event parsing or validation.
  #[allow(clippy::type_complexity)]
  fn from_event<IN: TokenMint, OUT: TokenMint>(
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
}

impl<X> SimulatedOperationExt for X {
  fn from_event<IN: TokenMint, OUT: TokenMint>(
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
    <Self as SimulatedOperation<IN, OUT>>::from_event(event)
  }
}
