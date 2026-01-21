//! Simulated operation trait for extracting quote data from events.
//!
//! Provides a unified interface for converting program events emitted during
//! transaction simulation into [`OperationOutput`] values.

mod exchange;
mod stability_pool;

use anchor_lang::{AnchorDeserialize, Discriminator};
use anyhow::Result;
use fix::typenum::Integer;
use hylo_idl::tokens::TokenMint;

use crate::token_operation::OperationOutput;

/// Extracts [`OperationOutput`] from a simulated transaction event.
///
/// This trait is the simulation counterpart to [`TokenOperation`]. While
/// `TokenOperation` computes quotes from protocol state using pure math,
/// `SimulatedOperation` extracts the same information from events emitted
/// during transaction simulation.
///
/// [`TokenOperation`]: crate::token_operation::TokenOperation
pub trait SimulatedOperation<IN: TokenMint, OUT: TokenMint> {
  type FeeExp: Integer;
  type Event: AnchorDeserialize + Discriminator;

  /// Extracts operation output from a simulation event.
  ///
  /// # Errors
  /// * Event field conversion
  /// * Operation-specific validation (e.g., levercoin in stability pool)
  fn from_event(
    event: &Self::Event,
  ) -> Result<OperationOutput<IN::Exp, OUT::Exp, Self::FeeExp>>;
}

/// Turbofish syntax for [`SimulatedOperation`].
pub trait SimulatedOperationExt {
  /// Extracts operation output from a simulation event.
  ///
  /// # Errors
  /// * Event field conversion
  /// * Operation-specific validation
  #[allow(clippy::type_complexity)]
  fn from_event<IN, OUT>(
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
    IN: TokenMint,
    OUT: TokenMint,
    <Self as SimulatedOperation<IN, OUT>>::FeeExp: Integer;
}

impl<X> SimulatedOperationExt for X {
  fn from_event<IN, OUT>(
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
    IN: TokenMint,
    OUT: TokenMint,
    <Self as SimulatedOperation<IN, OUT>>::FeeExp: Integer,
  {
    <Self as SimulatedOperation<IN, OUT>>::from_event(event)
  }
}
