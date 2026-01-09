//! Token operation trait for pure protocol math.

mod exchange;
mod stability_pool;

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use hylo_idl::tokens::TokenMint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationOutput {
  pub amount_out: u64,
  pub fee_amount: u64,
  pub fee_mint: Pubkey,
}

/// Pure math for token pair operations (mint/redeem/swap).
pub trait TokenOperation<IN: TokenMint, OUT: TokenMint> {
  /// # Errors
  /// * Stability mode restrictions
  /// * Math overflow
  fn compute(&self, amount_in: u64) -> Result<OperationOutput>;
}
