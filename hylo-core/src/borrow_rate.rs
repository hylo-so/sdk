use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::error::CoreError::{BorrowRateApply, BorrowRateValidation};
use crate::fees::controller::FeeExtract;
use crate::fees::interp::{FixInterp, Point};
use crate::rebalance::mode::RebalanceMode;
use crate::rebalance::pricing::narrow;

/// Per-epoch borrow rate for exogenous collateral without native yield.
#[derive(
  Copy,
  Clone,
  Debug,
  PartialEq,
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Serialize,
  Deserialize,
)]
pub struct BorrowRateCurveConfig {
  pub floor_bps: IFixValue64,
  pub ceil_bps: IFixValue64,
}

/// Maximum per-epoch rate (~30% annualized at 182 epochs/year)
const MAX_RATE: UFix64<N9> = UFix64::constant(1_648_352);

/// Maximum fee exacted against borrow rate
const MAX_FEE: UFix64<N4> = UFix64::constant(1_000);

impl BorrowRateCurveConfig {
  #[must_use]
  pub fn new(
    floor_bps: IFixValue64,
    ceil_bps: IFixValue64,
  ) -> BorrowRateCurveConfig {
    BorrowRateCurveConfig {
      floor_bps,
      ceil_bps,
    }
  }

  /// Minimum borrow rate.
  ///
  /// # Errors
  /// * Invalid rate data
  pub fn floor_bps(&self) -> Result<IFix64<N4>, CoreError> {
    Ok(self.floor_bps.try_into()?)
  }

  /// Maximum borrow rate.
  ///
  /// # Errors
  /// * Invalid rate data
  pub fn ceil_bps(&self) -> Result<IFix64<N4>, CoreError> {
    Ok(self.ceil_bps.try_into()?)
  }

  pub fn build_curve(&self) -> Result<FixInterp<2, N4>, CoreError> {
    let buy_1_start = RebalanceMode::BuyZone1
      .active_range()
      .start()
      .and_then(narrow)?;
    let buy_2_end = RebalanceMode::BuyZone2
      .active_range()
      .end()
      .and_then(narrow)?;
    let points: [Point<N4>; 2] = [
      Point::new(buy_1_start, self.floor_bps()?),
      Point::new(buy_2_end, self.ceil_bps()?),
    ];
    FixInterp::from_points(points)
  }
}
