//! Token operation trait for pure protocol math.

mod exchange;
mod stability_pool;

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use hylo_idl::tokens::TokenMint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationOutput {
  pub in_amount: u64,
  pub out_amount: u64,
  pub fee_amount: u64,
  pub fee_mint: Pubkey,
  pub fee_base: u64,
}

/// Pure math for token pair operations (mint/redeem/swap).
pub trait TokenOperation<IN: TokenMint, OUT: TokenMint> {
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput>;
}
