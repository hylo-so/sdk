use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace,
};
use fix::prelude::*;

use crate::error::CoreError;
use crate::error::CoreError::{
  LstLstPriceConversion, LstSolPriceConversion, LstSolPriceDelta,
  LstSolPriceEpochOrder, LstSolPriceOutdated, SolLstPriceConversion,
};
use crate::fees::controller::FeeExtract;
#[cfg(feature = "offchain")]
use crate::util::max_scaled_input;

/// Captures the true LST price in SOL for the current epoch.
#[derive(
  InitSpace,
  AnchorSerialize,
  AnchorDeserialize,
  Debug,
  PartialEq,
  Eq,
  Copy,
  Clone,
)]
pub struct LstSolPrice {
  pub price: UFixValue64,
  pub epoch: u64,
}

impl LstSolPrice {
  /// Constructs price for the given Solana epoch.
  #[must_use]
  pub fn new(price: UFixValue64, epoch: u64) -> LstSolPrice {
    LstSolPrice { price, epoch }
  }

  /// Returns a new [`LstSolPrice`] with the fee-discounted price
  /// at the same epoch.
  ///
  /// # Errors
  /// * Fee extraction arithmetic
  pub fn adjust_price(
    &self,
    fee: UFix64<N5>,
  ) -> Result<LstSolPrice, CoreError> {
    let price: UFix64<N9> = self.price.try_into()?;
    let extract = FeeExtract::new(fee, price)?;
    Ok(LstSolPrice::new(
      extract.amount_remaining.into(),
      self.epoch,
    ))
  }

  /// Computes difference between previous and current LST SOL price:
  ///  * Current epoch should be greater than the previous
  ///  * Price subtraction does not underflow
  pub fn checked_delta(
    &self,
    prev: &LstSolPrice,
  ) -> Result<UFix64<N9>, CoreError> {
    if self.epoch > prev.epoch {
      let cur: UFix64<N9> = self.price.try_into()?;
      let prev: UFix64<N9> = prev.price.try_into()?;
      cur.checked_sub(&prev).ok_or(LstSolPriceDelta)
    } else {
      Err(LstSolPriceEpochOrder)
    }
  }

  pub fn get_epoch_price(
    &self,
    current_epoch: u64,
  ) -> Result<UFix64<N9>, CoreError> {
    if current_epoch == self.epoch {
      Ok(self.price.try_into()?)
    } else {
      Err(LstSolPriceOutdated)
    }
  }

  /// Converts an amount of this LST to its underlying SOL.
  ///
  /// # Errors
  /// * Arithmetic overflow
  /// * Price outdated
  pub fn convert_lst_to_sol(
    &self,
    amount_lst: UFix64<N9>,
    current_epoch: u64,
  ) -> Result<UFix64<N9>, CoreError> {
    let lst_sol_price: UFix64<N9> = self.get_epoch_price(current_epoch)?;
    let sol = lst_sol_price
      .mul_div_floor(amount_lst, UFix64::one())
      .ok_or(LstSolPriceConversion)?;
    Ok(sol)
  }

  /// Inverse of [`convert_lst_to_sol`](Self::convert_lst_to_sol) under
  /// a SOL cap.
  ///
  /// # Errors
  /// * Degenerate price
  /// * Price outdated
  #[cfg(feature = "offchain")]
  pub fn max_lst_for_sol(
    &self,
    cap: UFix64<N9>,
    current_epoch: u64,
  ) -> Result<UFix64<N9>, CoreError> {
    let lst_sol_price: UFix64<N9> = self.get_epoch_price(current_epoch)?;
    max_scaled_input(cap, lst_sol_price, UFix64::one())
      .ok_or(LstSolPriceConversion)
  }

  /// Inverse of [`convert_lst_amount`](Self::convert_lst_amount) under
  /// an output cap.
  ///
  /// # Errors
  /// * Degenerate price
  /// * Price outdated
  #[cfg(feature = "offchain")]
  pub fn max_lst_for_lst(
    &self,
    cap: UFix64<N9>,
    current_epoch: u64,
    other: &LstSolPrice,
  ) -> Result<UFix64<N9>, CoreError> {
    let in_price = self.get_epoch_price(current_epoch)?;
    let out_price = other.get_epoch_price(current_epoch)?;
    max_scaled_input(cap, in_price, out_price).ok_or(LstLstPriceConversion)
  }

  /// Converts a SOL amount to this LST.
  ///
  /// # Errors
  /// * Arithmetic
  /// * Price outdated
  pub fn convert_sol_to_lst(
    &self,
    amount_sol: UFix64<N9>,
    current_epoch: u64,
  ) -> Result<UFix64<N9>, CoreError> {
    let lst_sol_price: UFix64<N9> = self.get_epoch_price(current_epoch)?;
    LstSolPrice::convert_sol_to_lst_inner(amount_sol, lst_sol_price)
      .ok_or(SolLstPriceConversion)
  }

  fn convert_sol_to_lst_inner(
    amount_sol: UFix64<N9>,
    lst_sol_price: UFix64<N9>,
  ) -> Option<UFix64<N9>> {
    (lst_sol_price != UFix64::zero())
      .then_some(amount_sol)
      .and_then(|amt| amt.mul_div_floor(UFix64::one(), lst_sol_price))
  }

  /// Converts an amount of this LST to another one using their prices.
  ///
  /// # Errors
  /// * Arithmetic
  /// * Price outdated
  pub fn convert_lst_amount(
    &self,
    current_epoch: u64,
    amount_lst: UFix64<N9>,
    other: &LstSolPrice,
  ) -> Result<UFix64<N9>, CoreError> {
    let in_price = self.get_epoch_price(current_epoch)?;
    let out_price = other.get_epoch_price(current_epoch)?;
    LstSolPrice::convert_lst_amount_inner(amount_lst, in_price, out_price)
      .ok_or(LstLstPriceConversion)
  }

  fn convert_lst_amount_inner(
    amount_lst: UFix64<N9>,
    in_price: UFix64<N9>,
    out_price: UFix64<N9>,
  ) -> Option<UFix64<N9>> {
    (out_price != UFix64::zero())
      .then_some(amount_lst)
      .and_then(|amt| amt.mul_div_floor(in_price, out_price))
  }
}

#[cfg(test)]
mod test {
  use fix::prelude::*;
  use proptest::prelude::*;

  use super::LstSolPrice;
  use crate::util::proptest::{lst_amount, lst_sol_price};

  /// Constrains inputs to only conversions which are feasible with `muldiv`.
  ///
  /// Given `out = in * in_price / out_price`
  /// Rearrange to `in * in_price = out * out_price`
  /// Assuming `out` is `u64::MAX`, ensure there will be no overflow.
  fn safe_conversion_inputs(
    amount: UFix64<N9>,
    in_price: UFix64<N9>,
    out_price: UFix64<N9>,
  ) -> Option<(UFix128<N9>, UFix128<N9>)> {
    let amount_wide = amount.widen::<u128>();
    let in_price_wide = in_price.widen::<u128>();
    let out_price_wide = out_price.widen::<u128>();
    let max_wide = UFix128::<N9>::new(u64::MAX.into());

    let lhs = amount_wide
      .checked_mul(&in_price_wide)
      .and_then(|m| m.checked_div(&UFix128::<N9>::one()))?;
    let rhs = out_price_wide
      .checked_mul(&max_wide)
      .and_then(|m| m.checked_div(&UFix128::<N9>::one()))?;

    (lhs <= rhs).then_some((lhs, out_price_wide))
  }

  proptest! {
      #[test]
      fn identity(
        price in lst_sol_price(),
        amount in lst_amount(),
      ) {
        prop_assume!(safe_conversion_inputs(amount, price, price).is_some());
        let epoch = 100;
        let lst = LstSolPrice::new(price.into(), epoch);
        let result = lst.convert_lst_amount(epoch, amount, &lst)?;
        prop_assert_eq!(result, amount);
      }

      #[test]
      fn floor_division(
        in_price in lst_sol_price(),
        out_price in lst_sol_price(),
        amount in lst_amount(),
      ) {
        let epoch = 100;
        let parts = safe_conversion_inputs(amount, in_price, out_price);
        prop_assume!(parts.is_some());

        // Compute LST output normally with `mul_div_floor` implementation.
        // `output = floor(amount * in_price / out_price)`
        let lst_in = LstSolPrice::new(in_price.into(), epoch);
        let lst_out = LstSolPrice::new(out_price.into(), epoch);
        let output = lst_in.convert_lst_amount(epoch, amount, &lst_out)?;

        // Distribute `out_price`
        // `output * out_price = floor(amount * in_price)`
        let (numerator, out_price_wide) = parts.ok_or(TestCaseError::fail("safe_conversion_inputs"))?;
        let output_wide = output.widen::<u128>();

        // Rearrange `floor` out of equation to get upper and lower bounds
        // `output * out_price <= amount * in_price < (output + 1) * out_price`
        let upper_bound: UFix128<N9> = output_wide
          .checked_add(&UFix128::one())
          .and_then(|o| o.checked_mul(&out_price_wide))
          .and_then(|o| o.checked_div(&UFix128::<N9>::one()))
          .ok_or(TestCaseError::fail("upper_bound"))?;
        let lower_bound: UFix128<N9> = output_wide
          .checked_mul(&out_price_wide)
          .and_then(|o| o.checked_div(&UFix128::<N9>::one()))
          .ok_or(TestCaseError::fail("lower_bound"))?;

        // Check that floored result is within bounds
        prop_assert!(numerator >= lower_bound, "amount * in_price >= output * out_price");
        prop_assert!(numerator < upper_bound, "amount * in_price < (output + 1) * out_price");
      }

      #[test]
      fn round_trip_loss(
        price_a in lst_sol_price(),
        price_b in lst_sol_price(),
        amount in lst_amount(),
      ) {
        let epoch = 100;
        let parts = safe_conversion_inputs(amount, price_a, price_b);
        prop_assume!(parts.is_some());
        let lst_a = LstSolPrice::new(price_a.into(), epoch);
        let lst_b = LstSolPrice::new(price_b.into(), epoch);

        // Convert to B, then revert back using A's price
        let convert_to_b = lst_a.convert_lst_amount(epoch, amount, &lst_b)?;
        let convert_back_to_a = lst_b.convert_lst_amount(epoch, convert_to_b, &lst_a)?;

        // The conversion cannot exceed the original input amount because of floor
        prop_assert!(
          convert_back_to_a <= amount,
          "round trip gained value: {:?} -> {:?}",
          amount, convert_back_to_a
        );
      }
  }
}
