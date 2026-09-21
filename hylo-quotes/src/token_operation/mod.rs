//! Token operation trait for pure protocol math.
//!
//! Three tiers, in increasing looseness: gated
//! ([`TokenOperation::compute_output`]),
//! ungated ([`TokenOperation::compute_output_ungated`]), and indicative
//! ([`TokenOperation::compute_output_indicative`]).

mod earn_pool;
mod exchange;
mod redemption_rate;

use anchor_lang::prelude::Pubkey;
use fix::prelude::{CheckedAdd, UFix64, N6, N9};
use fix::typenum::Integer;
use hylo_core::calculus::{positive, positive_rate};
use hylo_core::error::CoreError;
use hylo_idl::tokens::TokenMint;
pub use redemption_rate::{RedemptionLane, RedemptionRate};

fn gate(condition: bool, error: CoreError) -> Result<(), CoreError> {
  condition.then_some(()).ok_or(error)
}

/// Smallest input past the largest zero-output input.
fn past_zero<Exp: Integer>(
  zero_ceiling: UFix64<Exp>,
) -> Result<UFix64<Exp>, CoreError> {
  zero_ceiling
    .checked_add(&UFix64::new(1))
    .ok_or(CoreError::MinInputOverflow)
}

/// Marginal rate of a fee-flat route in atoms. The output is linear in
/// the input, so the realized atom ratio is the exact derivative.
///
/// # Errors
/// * Non-finite or non-positive rate (zero input or output)
pub(crate) fn linear_rate<InExp: Integer, OutExp: Integer>(
  in_amount: UFix64<InExp>,
  out_amount: UFix64<OutExp>,
) -> Result<f64, CoreError> {
  let rate = positive(out_amount)?.get() / positive(in_amount)?.get();
  positive_rate(atom_rate::<InExp, OutExp>(rate))
}

/// Scales a token-level marginal rate to atoms:
/// `rate * 10^(out_decimals - in_decimals)`.
fn atom_rate<InExp: Integer, OutExp: Integer>(token_rate: f64) -> f64 {
  token_rate * 10f64.powi(InExp::to_i32() - OutExp::to_i32())
}

/// Which collateral ratio priced a stablecoin redeem fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FeeBasis {
  /// Fee at the projected post-trade CR: what the strict math uses.
  /// Flat-fee lanes (USDC) have no CR to project, and always report
  /// this variant.
  CurrentCr,
  /// Projected CR is above the redeem fee-curve domain. Fee at the
  /// domain edge. The route cannot execute in this state.
  RedeemMaxCr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OperationOutput<InExp: Integer, OutExp: Integer, FeeExp: Integer> {
  pub in_amount: UFix64<InExp>,
  pub out_amount: UFix64<OutExp>,
  pub fee_amount: UFix64<FeeExp>,
  pub fee_mint: Pubkey,
  pub fee_base: UFix64<FeeExp>,
  pub marginal_rate: f64,
}

pub type MintOperationOutput = OperationOutput<N9, N6, N9>;
pub type RedeemOperationOutput = OperationOutput<N6, N9, N9>;
pub type SwapOperationOutput = OperationOutput<N6, N6, N6>;
pub type LstSwapOperationOutput = OperationOutput<N9, N9, N9>;

pub trait TokenOperation<IN: TokenMint, OUT: TokenMint> {
  type FeeExp: Integer;

  /// State-only route gates; an error means the route is closed.
  ///
  /// # Errors
  /// * Route gated in current state (paused, mode-disabled, unharvested)
  fn preconditions(&self) -> Result<(), CoreError>;

  /// Pure math for the operation, skipping [`Self::preconditions`].
  ///
  /// # Errors
  /// * Underlying arithmetic
  fn compute_output_ungated(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<OperationOutput<IN::Exp, OUT::Exp, Self::FeeExp>, CoreError>;

  /// Input ceiling for the route, skipping [`Self::preconditions`].
  ///
  /// # Errors
  /// * Underlying arithmetic
  fn max_input_ungated(&self) -> Result<UFix64<IN::Exp>, CoreError>;

  /// Smallest input yielding at least one output atom, skipping
  /// [`Self::preconditions`].
  ///
  /// # Errors
  /// * Underlying arithmetic
  fn min_input_ungated(&self) -> Result<UFix64<IN::Exp>, CoreError>;

  /// Pure math to complete a token pair operation (mint/redeem/swap).
  ///
  /// # Errors
  /// * Route gated in current state or underlying arithmetic
  fn compute_output(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<OperationOutput<IN::Exp, OUT::Exp, Self::FeeExp>, CoreError> {
    self.preconditions()?;
    self.compute_output_ungated(amount_in)
  }

  /// Largest executable input for the route in the current state.
  ///
  /// # Errors
  /// * Route gated in current state or underlying arithmetic
  /// * Ceiling below [`Self::min_input_ungated`]
  fn max_input(&self) -> Result<UFix64<IN::Exp>, CoreError> {
    self.preconditions()?;
    let max = self.max_input_ungated()?;
    gate(
      self.min_input_ungated()? <= max,
      CoreError::MinInputExceedsMax,
    )?;
    Ok(max)
  }

  /// Smallest input the route turns into output in the current state.
  ///
  /// # Errors
  /// * Route gated in current state or underlying arithmetic
  fn min_input(&self) -> Result<UFix64<IN::Exp>, CoreError> {
    self.preconditions()?;
    self.min_input_ungated()
  }

  /// Output as if the route could execute. Skips
  /// [`Self::preconditions`]. The default is
  /// [`Self::compute_output_ungated`]. A route whose math has a fee-curve
  /// domain overrides it to price a state outside the domain at the
  /// domain edge (today: `HYUSD -> LST` and `HYUSD -> Exo`). A rate
  /// reference, never an executable quote. Equal to
  /// [`Self::compute_output_ungated`] whenever that succeeds.
  ///
  /// # Errors
  /// * Underlying arithmetic
  fn compute_output_indicative(
    &self,
    amount_in: UFix64<IN::Exp>,
  ) -> Result<OperationOutput<IN::Exp, OUT::Exp, Self::FeeExp>, CoreError> {
    self.compute_output_ungated(amount_in)
  }
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

  /// Turbofish form of [`TokenOperation::compute_output_indicative`].
  ///
  /// # Errors
  /// * Underlying arithmetic
  fn indicative_output<IN, OUT>(
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

  fn indicative_output<IN, OUT>(
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
    TokenOperation::<IN, OUT>::compute_output_indicative(self, amount_in)
  }
}
