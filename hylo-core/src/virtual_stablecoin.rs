use anchor_lang::prelude::*;
use fix::prelude::*;

use crate::error::CoreError::{
  BurnUnderflow, BurnZero, MintOverflow, MintZero,
};

/// Simple counter representing the supply of a "virtual" stablecoin.
#[derive(Debug, Clone, Copy, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub struct VirtualStablecoin {
  pub(crate) supply: UFixValue64,
}

impl Default for VirtualStablecoin {
  fn default() -> Self {
    Self::new()
  }
}

impl VirtualStablecoin {
  #[must_use]
  pub fn new() -> VirtualStablecoin {
    VirtualStablecoin {
      supply: UFix64::<N6>::zero().into(),
    }
  }

  /// Lifts serialized supply to typed Fix.
  ///
  /// # Errors
  /// * Invalid supply data cannot convert to typed
  pub fn supply(&self) -> Result<UFix64<N6>> {
    self.supply.try_into()
  }

  /// Increases the supply of the virtual stablecoin.
  ///
  /// # Errors
  /// * State validation
  /// * Overflow
  pub fn mint(&mut self, amount: UFix64<N6>) -> Result<()> {
    if amount > UFix64::zero() {
      let current_supply = self.supply()?;
      let new_supply =
        current_supply.checked_add(&amount).ok_or(MintOverflow)?;
      self.supply = new_supply.into();
      Ok(())
    } else {
      Err(MintZero.into())
    }
  }

  /// Decreases the supply of the virtual stablecoin.
  ///
  /// # Errors
  /// * State validation
  /// * Underflow
  pub fn burn(&mut self, amount: UFix64<N6>) -> Result<()> {
    if amount > UFix64::zero() {
      let current_supply = self.supply()?;
      let new_supply =
        current_supply.checked_sub(&amount).ok_or(BurnUnderflow)?;
      self.supply = new_supply.into();
      Ok(())
    } else {
      Err(BurnZero.into())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn setup_virtual_stablecoin() -> VirtualStablecoin {
    VirtualStablecoin::new()
  }

  #[test]
  fn new_initializes_zero_supply() -> Result<()> {
    let stablecoin = setup_virtual_stablecoin();
    assert_eq!(stablecoin.supply()?, UFix64::zero());
    Ok(())
  }

  #[test]
  fn mint_increases_supply() -> Result<()> {
    let mut stablecoin = setup_virtual_stablecoin();
    let five = UFix64::new(5_000_000);
    stablecoin.mint(five)?;
    assert_eq!(stablecoin.supply()?, five);
    Ok(())
  }

  #[test]
  fn burn_decreases_supply() -> Result<()> {
    let mut stablecoin = setup_virtual_stablecoin();
    let ten = UFix64::new(10_000_000);
    let three = UFix64::new(3_000_000);
    let seven = UFix64::new(7_000_000);
    stablecoin.mint(ten)?;
    stablecoin.burn(three)?;
    assert_eq!(stablecoin.supply()?, seven);
    Ok(())
  }

  #[test]
  fn mint_then_burn_returns_to_zero() -> Result<()> {
    let mut stablecoin = setup_virtual_stablecoin();
    let five = UFix64::new(5_000_000);
    stablecoin.mint(five)?;
    stablecoin.burn(five)?;
    assert_eq!(stablecoin.supply()?, UFix64::zero());
    Ok(())
  }

  #[test]
  fn mint_overflow_error() -> Result<()> {
    let mut stablecoin = setup_virtual_stablecoin();
    stablecoin.mint(UFix64::one())?;
    let result = stablecoin.mint(UFix64::new(u64::MAX));
    assert!(result.is_err_and(|e| e == MintOverflow.into()));
    Ok(())
  }

  #[test]
  fn burn_underflow_error() -> Result<()> {
    let mut stablecoin = setup_virtual_stablecoin();
    stablecoin.mint(UFix64::one())?;
    let result = stablecoin.burn(UFix64::new(2_000_000));
    assert!(result.is_err_and(|e| e == BurnUnderflow.into()));
    Ok(())
  }

  #[test]
  fn mint_zero_error() {
    let mut stablecoin = setup_virtual_stablecoin();
    let result = stablecoin.mint(UFix64::zero());
    assert!(result.is_err_and(|e| e == MintZero.into()));
  }

  #[test]
  fn burn_zero_error() -> Result<()> {
    let mut stablecoin = setup_virtual_stablecoin();
    stablecoin.mint(UFix64::one())?;
    let result = stablecoin.burn(UFix64::zero());
    assert!(result.is_err_and(|e| e == BurnZero.into()));
    Ok(())
  }
}
