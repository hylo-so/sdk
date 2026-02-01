use anchor_lang::*;
use fix::prelude::*;

use crate::error::CoreError;
use crate::fee_controller::FeeExtract;
use crate::interp::{FixInterp, PointValue};

/// Downconvert CR from `N9` unsigned to `N5` signed for curve lookup.
///
/// # Errors
/// * `CollateralRatioConversion` on `i64` overflow.
pub fn narrow_cr(cr: UFix64<N9>) -> Result<IFix64<N5>> {
  cr.convert::<N5>()
    .narrow::<i64>()
    .ok_or(CoreError::CollateralRatioConversion.into())
}

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
    let cr = narrow_cr(ucr)?;
    let fee = self
      .fee_inner(cr)?
      .narrow()
      .ok_or(CoreError::InterpFeeConversion)?;
    FeeExtract::new(fee, amount_in)
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

#[cfg(test)]
mod tests {
  use fix::typenum::Integer;
  use proptest::prelude::*;
  use proptest::test_runner::TestCaseResult;

  use super::*;
  use crate::error::CoreError;
  use crate::fee_curves::{MINT_FEE_EXP_DECAY, REDEEM_FEE_LN};
  use crate::util::proptest::*;

  fn collateral_ratio() -> BoxedStrategy<UFix64<N9>> {
    (0u64..4_000_000_000u64).prop_map(UFix64::new).boxed()
  }

  fn mint_fees() -> InterpolatedMintFees {
    InterpolatedMintFees::new(MINT_FEE_EXP_DECAY.map(Into::into))
  }

  fn redeem_fees() -> InterpolatedRedeemFees {
    InterpolatedRedeemFees::new(REDEEM_FEE_LN.map(Into::into))
  }

  fn assert_conservation<Exp: Integer>(
    extract: &FeeExtract<Exp>,
    amount: UFix64<Exp>,
    cr: UFix64<N9>,
  ) -> TestCaseResult {
    prop_assert_eq!(
      extract
        .fees_extracted
        .checked_add(&extract.amount_remaining),
      Some(amount),
      "Fee conservation violated at CR {:?}",
      cr,
    );
    Ok(())
  }

  fn assert_nonzero_fee<Exp: Integer>(
    extract: &FeeExtract<Exp>,
    cr: UFix64<N9>,
    amount: UFix64<Exp>,
  ) -> TestCaseResult {
    prop_assert!(
      extract.fees_extracted > UFix64::new(0),
      "Precision loss: zero fees at CR {:?} for amount {:?}",
      cr,
      amount,
    );
    Ok(())
  }

  fn assert_mint_fee<Exp: Integer>(
    cr: UFix64<N9>,
    amount: UFix64<Exp>,
  ) -> TestCaseResult {
    let fees = mint_fees();
    let interp = fees.curve().map_err(|e| {
      TestCaseError::fail(format!("Curve construction failed: {e}"))
    })?;
    let cr_n5 = narrow_cr(cr)
      .map_err(|e| TestCaseError::fail(format!("CR narrowing failed: {e}")))?;
    match fees.apply_fee(cr, amount) {
      Ok(extract) => {
        assert_conservation(&extract, amount, cr)?;
        assert_nonzero_fee(&extract, cr, amount)?;
      }
      Err(e) => prop_assert!(
        cr_n5 < interp.x_min()
          && e == CoreError::NoValidStablecoinMintFee.into(),
        "Mint fee rejected in-domain CR {:?}: {}",
        cr,
        e,
      ),
    }
    Ok(())
  }

  fn assert_redeem_fee<Exp: Integer>(
    cr: UFix64<N9>,
    amount: UFix64<Exp>,
  ) -> TestCaseResult {
    let fees = redeem_fees();
    let interp = fees.curve().map_err(|e| {
      TestCaseError::fail(format!("Curve construction failed: {e}"))
    })?;
    let cr_n5 = narrow_cr(cr)
      .map_err(|e| TestCaseError::fail(format!("CR narrowing failed: {e}")))?;
    let extract = fees.apply_fee(cr, amount).map_err(|e| {
      TestCaseError::fail(format!(
        "Redeem fee should always work at CR {cr:?}: {e}"
      ))
    })?;
    assert_conservation(&extract, amount, cr)?;
    if cr_n5 > interp.x_min() {
      assert_nonzero_fee(&extract, cr, amount)?;
    }
    Ok(())
  }

  proptest! {
    #[test]
    fn mint_apply_fee_lst(
      cr in collateral_ratio(),
      amount in lst_amount(),
    ) {
      assert_mint_fee(cr, amount)?;
    }

    #[test]
    fn mint_apply_fee_token(
      cr in collateral_ratio(),
      amount in token_amount(),
    ) {
      assert_mint_fee(cr, amount)?;
    }

    #[test]
    fn redeem_apply_fee_lst(
      cr in collateral_ratio(),
      amount in lst_amount(),
    ) {
      assert_redeem_fee(cr, amount)?;
    }

    #[test]
    fn redeem_apply_fee_token(
      cr in collateral_ratio(),
      amount in token_amount(),
    ) {
      assert_redeem_fee(cr, amount)?;
    }
  }
}
