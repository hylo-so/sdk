use anchor_lang::*;
use fix::prelude::*;

use crate::error::CoreError;
use crate::fee_controller::FeeExtract;
use crate::interp::{FixInterp, PointValue};

/// Interpolated fee curve controller.
/// Implementors define boundary behavior via [`fee_inner`].
pub trait InterpolatedFeeController {
  /// Builds the interpolator from the underlying curve.
  ///
  /// # Errors
  /// * Curve validation (insufficient points, non-monotonic)
  fn curve(&self) -> Result<FixInterp<20, N5>>;

  /// Compute fee for collateral ratio from underlying curve.
  ///
  /// # Errors
  /// * Domain or arithmetic errors specific to the fee type
  fn fee_inner(&self, cr: IFix64<N5>) -> Result<IFix64<N5>>;

  /// Applies the interpolated fee to an input amount.
  /// Downconverts CR to 5 decimal places to guard against narrowing
  /// errors.
  ///
  /// # Errors
  /// * CR conversion
  /// * Domain error
  /// * Fee extraction arithmetic
  fn apply_fee<InExp>(
    &self,
    ucr: UFix64<N9>,
    amount_in: UFix64<InExp>,
  ) -> Result<FeeExtract<InExp>> {
    let cr: IFix64<N5> = ucr
      .convert::<N5>()
      .narrow::<i64>()
      .ok_or(CoreError::CollateralRatioConversion)?;
    let fee = self
      .fee_inner(cr)?
      .narrow()
      .ok_or(CoreError::InterpFeeConversion)?;
    FeeExtract::new_n5(fee, amount_in)
  }
}

pub struct InterpolatedMintFees {
  curve: [PointValue; 20],
}

impl InterpolatedMintFees {
  #[must_use]
  pub fn new(curve: [PointValue; 20]) -> InterpolatedMintFees {
    InterpolatedMintFees { curve }
  }
}

impl InterpolatedFeeController for InterpolatedMintFees {
  fn curve(&self) -> Result<FixInterp<20, N5>> {
    FixInterp::from_values(self.curve)
  }

  fn fee_inner(&self, cr: IFix64<N5>) -> Result<IFix64<N5>> {
    let interp = self.curve()?;
    if cr < interp.x_min() {
      Err(CoreError::NoValidStablecoinMintFee.into())
    } else if cr > interp.x_max() {
      Ok(interp.y_max())
    } else {
      interp.interpolate(cr)
    }
  }
}

pub struct InterpolatedRedeemFees {
  curve: [PointValue; 20],
}

impl InterpolatedRedeemFees {
  #[must_use]
  pub fn new(curve: [PointValue; 20]) -> InterpolatedRedeemFees {
    InterpolatedRedeemFees { curve }
  }
}

impl InterpolatedFeeController for InterpolatedRedeemFees {
  fn curve(&self) -> Result<FixInterp<20, N5>> {
    FixInterp::from_values(self.curve)
  }

  fn fee_inner(&self, cr: IFix64<N5>) -> Result<IFix64<N5>> {
    let interp = self.curve()?;
    if cr < interp.x_min() {
      Ok(interp.y_min())
    } else if cr > interp.x_max() {
      Ok(interp.y_max())
    } else {
      interp.interpolate(cr)
    }
  }
}
