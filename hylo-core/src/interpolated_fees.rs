use anchor_lang::*;
use fix::prelude::*;

use crate::error::CoreError;
use crate::fee_controller::FeeExtract;
use crate::interp::FixInterp;

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
/// Implementors define boundary behavior via `fee_inner`.
pub trait InterpolatedFeeController<const RES: usize> {
  /// Returns a reference to the underlying interpolator.
  fn curve(&self) -> &FixInterp<RES, N5>;

  /// Compute fee for collateral ratio from underlying curve.
  ///
  /// # Errors
  /// * Domain or arithmetic errors
  fn fee_inner(&self, cr: IFix64<N5>) -> Result<IFix64<N5>>;

  /// Applies the interpolated fee to an input amount.
  ///
  /// # Errors
  /// * CR conversion, domain, or fee extraction arithmetic
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

  /// Minimum collateral ratio in the curve's domain.
  fn cr_floor(&self) -> Result<UFix64<N2>> {
    self
      .curve()
      .x_min()
      .narrow()
      .and_then(UFix64::checked_convert::<N2>)
      .ok_or(CoreError::InterpFeeConversion.into())
  }
}

#[derive(Clone)]
pub struct InterpolatedMintFees {
  curve: FixInterp<21, N5>,
}

impl InterpolatedMintFees {
  #[must_use]
  pub fn new(curve: FixInterp<21, N5>) -> InterpolatedMintFees {
    InterpolatedMintFees { curve }
  }
}

impl InterpolatedFeeController<21> for InterpolatedMintFees {
  fn curve(&self) -> &FixInterp<21, N5> {
    &self.curve
  }

  fn fee_inner(&self, cr: IFix64<N5>) -> Result<IFix64<N5>> {
    let interp = self.curve();
    if cr < interp.x_min() {
      Err(CoreError::NoValidStablecoinMintFee.into())
    } else if cr > interp.x_max() {
      Ok(interp.y_max())
    } else {
      interp.interpolate(cr)
    }
  }
}

#[derive(Clone)]
pub struct InterpolatedRedeemFees {
  curve: FixInterp<20, N5>,
}

impl InterpolatedRedeemFees {
  #[must_use]
  pub fn new(curve: FixInterp<20, N5>) -> InterpolatedRedeemFees {
    InterpolatedRedeemFees { curve }
  }
}

impl InterpolatedFeeController<20> for InterpolatedRedeemFees {
  fn curve(&self) -> &FixInterp<20, N5> {
    &self.curve
  }

  fn fee_inner(&self, cr: IFix64<N5>) -> Result<IFix64<N5>> {
    let interp = self.curve();
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
  use crate::fee_curves::{MINT_FEE_INV, REDEEM_FEE_LN};
  use crate::util::proptest::*;

  fn collateral_ratio() -> BoxedStrategy<UFix64<N9>> {
    (0u64..4_000_000_000u64).prop_map(UFix64::new).boxed()
  }

  fn mint_fees() -> InterpolatedMintFees {
    let curve = FixInterp::from_points_unchecked(*MINT_FEE_INV);
    InterpolatedMintFees::new(curve)
  }

  fn redeem_fees() -> InterpolatedRedeemFees {
    let curve = FixInterp::from_points_unchecked(*REDEEM_FEE_LN);
    InterpolatedRedeemFees::new(curve)
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

  fn assert_mint_fee<Exp: Integer>(
    cr: UFix64<N9>,
    amount: UFix64<Exp>,
  ) -> TestCaseResult {
    let fees = mint_fees();
    let cr_n5 = narrow_cr(cr)
      .map_err(|e| TestCaseError::fail(format!("CR narrowing failed: {e}")))?;
    match fees.apply_fee(cr, amount) {
      Ok(extract) => assert_conservation(&extract, amount, cr),
      Err(e) => {
        prop_assert!(
          cr_n5 < fees.curve().x_min()
            && e == CoreError::NoValidStablecoinMintFee.into(),
          "Mint fee rejected in-domain CR {:?}: {}",
          cr,
          e,
        );
        Ok(())
      }
    }
  }

  fn assert_redeem_fee<Exp: Integer>(
    cr: UFix64<N9>,
    amount: UFix64<Exp>,
  ) -> TestCaseResult {
    let fees = redeem_fees();
    let extract = fees.apply_fee(cr, amount).map_err(|e| {
      TestCaseError::fail(format!(
        "Redeem fee should always work at CR {cr:?}: {e}"
      ))
    })?;
    assert_conservation(&extract, amount, cr)
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

    #[test]
    fn mint_fee_decreases_with_cr(
      cr_a in collateral_ratio(),
      cr_b in collateral_ratio(),
      amount in lst_amount(),
    ) {
      let (cr_high, cr_low) = if cr_a > cr_b {
        (cr_a, cr_b)
      } else {
        (cr_b, cr_a)
      };
      prop_assume!(cr_high > cr_low);

      let fees = mint_fees();
      if let (Ok(high), Ok(low)) = (
        fees.apply_fee(cr_high, amount),
        fees.apply_fee(cr_low, amount),
      ) {
        prop_assert!(
          high.fees_extracted <= low.fees_extracted,
          "fee({cr_high:?}) = {:?} > fee({cr_low:?}) = {:?}",
          high.fees_extracted, low.fees_extracted,
        );
      } else {
        Err(TestCaseError::reject("CR below threshold"))?;
      }
    }

    #[test]
    fn redeem_fee_increases_with_cr(
      cr_a in collateral_ratio(),
      cr_b in collateral_ratio(),
      amount in lst_amount(),
    ) {
      let (cr_high, cr_low) = if cr_a > cr_b {
        (cr_a, cr_b)
      } else {
        (cr_b, cr_a)
      };
      prop_assume!(cr_high > cr_low);

      let fees = redeem_fees();
      let high = fees.apply_fee(cr_high, amount).map_err(|e| {
        TestCaseError::fail(format!("at {cr_high:?}: {e}"))
      })?;
      let low = fees.apply_fee(cr_low, amount).map_err(|e| {
        TestCaseError::fail(format!("at {cr_low:?}: {e}"))
      })?;

      prop_assert!(
        high.fees_extracted >= low.fees_extracted,
        "fee({cr_high:?}) = {:?} < fee({cr_low:?}) = {:?}",
        high.fees_extracted, low.fees_extracted,
      );
    }
  }
}
