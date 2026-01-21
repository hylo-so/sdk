//! Token operation trait for pure protocol math.

mod exchange;
mod stability_pool;

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use fix::prelude::{UFix64, N6, N9};
use fix::typenum::Integer;
use hylo_idl::tokens::TokenMint;

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
  fn compute_quote(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<OperationOutput<IN::Exp, OUT::Exp, Self::FeeExp>>;
}

/// Turbofish syntax for [`TokenOperation`].
#[allow(clippy::type_complexity)]
pub trait TokenOperationExt {
  /// Computes quote for a token pair operation.
  ///
  /// # Errors
  /// * Stability mode restrictions
  /// * Math overflow
  fn compute_quote<IN, OUT>(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<
    OperationOutput<
      IN::Exp,
      OUT::Exp,
      <Self as TokenOperation<IN, OUT>>::FeeExp,
    >,
  >
  where
    Self: TokenOperation<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint,
    <Self as TokenOperation<IN, OUT>>::FeeExp: Integer;
}

impl<X> TokenOperationExt for X {
  fn compute_quote<IN, OUT>(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<
    OperationOutput<
      IN::Exp,
      OUT::Exp,
      <Self as TokenOperation<IN, OUT>>::FeeExp,
    >,
  >
  where
    Self: TokenOperation<IN, OUT>,
    IN: TokenMint,
    OUT: TokenMint,
    <Self as TokenOperation<IN, OUT>>::FeeExp: Integer,
  {
    TokenOperation::<IN, OUT>::compute_quote(self, amount_in)
  }
}
