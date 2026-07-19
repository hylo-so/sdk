//! Token operation trait for pure protocol math.

mod earn_pool;
mod exchange;

use anchor_lang::prelude::Pubkey;
use fix::prelude::{UFix64, N6, N9};
use fix::typenum::Integer;
use hylo_core::calculus::positive_rate;
use hylo_core::error::CoreError;
use hylo_idl::tokens::TokenMint;

fn gate(condition: bool, error: CoreError) -> Result<(), CoreError> {
  condition.then_some(()).ok_or(error)
}

/// Marginal rate of a fee-flat route in atoms. The output is linear in
/// the input, so the realized atom ratio is the exact derivative.
///
/// # Errors
/// * Non-finite or non-positive rate (zero input or output)
#[allow(clippy::cast_precision_loss)]
pub(crate) fn linear_rate<InExp: Integer, OutExp: Integer>(
  in_amount: UFix64<InExp>,
  out_amount: UFix64<OutExp>,
) -> Result<f64, CoreError> {
  positive_rate(out_amount.bits as f64 / in_amount.bits as f64)
}

/// Scales a token-level marginal rate to atoms:
/// `rate * 10^(out_decimals - in_decimals)`.
fn atom_rate<InExp: Integer, OutExp: Integer>(token_rate: f64) -> f64 {
  token_rate * 10f64.powi(InExp::to_i32() - OutExp::to_i32())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OperationOutput<InExp: Integer, OutExp: Integer, FeeExp: Integer> {
  pub in_amount: UFix64<InExp>,
  pub out_amount: UFix64<OutExp>,
  pub fee_amount: UFix64<FeeExp>,
  pub fee_mint: Pubkey,
  pub fee_base: UFix64<FeeExp>,
  /// Marginal output rate `f'(in_amount)` in atoms — Titan's
  /// `QuoteResult.price`.
  pub marginal_rate: f64,
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
