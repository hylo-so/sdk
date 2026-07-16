//! Token operation trait for pure protocol math.

mod earn_pool;
mod exchange;

use anchor_lang::prelude::Pubkey;
use fix::prelude::{UFix64, N6, N9};
use fix::typenum::Integer;
use hylo_core::error::CoreError;
use hylo_idl::tokens::TokenMint;

/// Maps a failed gate condition to its error.
pub(crate) fn gate(condition: bool, error: CoreError) -> Result<(), CoreError> {
  condition.then_some(()).ok_or(error)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationOutput<InExp: Integer, OutExp: Integer, FeeExp: Integer> {
  pub in_amount: UFix64<InExp>,
  pub out_amount: UFix64<OutExp>,
  pub fee_amount: UFix64<FeeExp>,
  pub fee_mint: Pubkey,
  pub fee_base: UFix64<FeeExp>,
}

pub type MintOperationOutput = OperationOutput<N9, N6, N9>;
pub type RedeemOperationOutput = OperationOutput<N6, N9, N9>;
pub type SwapOperationOutput = OperationOutput<N6, N6, N6>;
pub type LstSwapOperationOutput = OperationOutput<N9, N9, N9>;

pub trait TokenOperation<IN: TokenMint, OUT: TokenMint> {
  type FeeExp: Integer;

  /// Pure math to complete a token pair operation (mint/redeem/swap).
  ///
  /// # Errors
  /// * Underlying arithmetic
  fn compute_output(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<OperationOutput<IN::Exp, OUT::Exp, Self::FeeExp>, CoreError>;
}

/// Turbofish helper for [`TokenOperation`].
pub trait TokenOperationExt {
  /// # Errors
  /// * Arithmetic or mode restrictions.
  fn output<IN, OUT>(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<
    OperationOutput<
      IN::Exp,
      OUT::Exp,
      <Self as TokenOperation<IN, OUT>>::FeeExp,
    >,
    CoreError,
  >
  where
    Self: TokenOperation<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint,
    <Self as TokenOperation<IN, OUT>>::FeeExp: Integer;
}

impl<X> TokenOperationExt for X {
  fn output<IN, OUT>(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<
    OperationOutput<
      IN::Exp,
      OUT::Exp,
      <Self as TokenOperation<IN, OUT>>::FeeExp,
    >,
    CoreError,
  >
  where
    Self: TokenOperation<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint,
    <Self as TokenOperation<IN, OUT>>::FeeExp: Integer,
  {
    TokenOperation::<IN, OUT>::compute_output(self, amount_in)
  }
}
