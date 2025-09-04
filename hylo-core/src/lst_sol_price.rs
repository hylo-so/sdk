use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::{
  LstSolPriceConversion, LstSolPriceDelta, LstSolPriceEpochOrder,
  LstSolPriceOutdated,
};

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

  /// Computes difference between previous and current LST SOL price:
  ///  * Current epoch should be greater than the previous
  ///  * Price subtraction does not underflow
  pub fn checked_delta(&self, prev: &LstSolPrice) -> Result<UFix64<N9>> {
    if self.epoch > prev.epoch {
      let cur: UFix64<N9> = self.price.try_into()?;
      let prev: UFix64<N9> = prev.price.try_into()?;
      cur.checked_sub(&prev).ok_or(LstSolPriceDelta.into())
    } else {
      Err(LstSolPriceEpochOrder.into())
    }
  }

  pub fn get_epoch_price(&self, current_epoch: u64) -> Result<UFix64<N9>> {
    if current_epoch == self.epoch {
      self.price.try_into()
    } else {
      Err(LstSolPriceOutdated.into())
    }
  }

  pub fn convert_sol(
    &self,
    amount_lst: UFix64<N9>,
    current_epoch: u64,
  ) -> Result<UFix64<N9>> {
    let lst_sol_price: UFix64<N9> = self.get_epoch_price(current_epoch)?;
    let sol = lst_sol_price
      .mul_div_floor(amount_lst, UFix64::one())
      .ok_or(LstSolPriceConversion)?;
    Ok(sol)
  }
}
