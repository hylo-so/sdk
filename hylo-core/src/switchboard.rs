use anchor_lang::prelude::Result;
use fix::prelude::*;
use fix::typenum::Integer;
use switchboard_on_demand::SwitchboardQuote;

use crate::error::CoreError::{
  SwitchboardOracleInvalidValue, SwitchboardOraclePriceRange,
  SwitchboardOracleStale,
};
use crate::oracle::{OracleConfig, PriceRange};
use crate::solana_clock::SolanaClock;

/// Fetches price range from a Switchboard oracle with validations.
/// Uses the common OracleConfig and PriceRange types.
/// Note: Switchboard doesn't provide std_dev, so we assume zero confidence interval.
pub fn query_switchboard_price<Exp: Integer, C: SolanaClock>(
  clock: &C,
  quote: &SwitchboardQuote,
  OracleConfig {
    interval_secs,
    conf_tolerance: _,
  }: OracleConfig<Exp>,
) -> Result<PriceRange<Exp>>
where
  UFix64<Exp>: FixExt,
{
  // Validate feed staleness
  validate_staleness(quote, interval_secs, clock)?;

  // Get the first feed from the quote
  let feed = quote
    .feeds
    .first()
    .ok_or(SwitchboardOracleInvalidValue)?;

  // Get the price value from the feed
  let value = feed.value();

  // Convert to fixed point with the correct exponent
  let spot_price = decimal_to_fixed::<Exp>(value)?;

  // Switchboard doesn't provide std_dev, so we use zero
  let spot_std_dev = UFix64::zero();

  // Build price range from median and std dev (zero confidence interval for Switchboard)
  PriceRange::from_conf(spot_price, spot_std_dev)
}

/// Validates that the feed is not stale based on last update timestamp
fn validate_staleness<C: SolanaClock>(
  quote: &SwitchboardQuote,
  max_staleness_secs: u64,
  clock: &C,
) -> Result<()> {
  let current_slot = clock.slot();
  let last_update = quote.slot;

  // Convert max_staleness_secs to slots (200ms per slot)
  let max_staleness_slots = (max_staleness_secs * 1000) / 200;

  if current_slot.saturating_sub(last_update) <= max_staleness_slots {
    Ok(())
  } else {
    Err(SwitchboardOracleStale.into())
  }
}

/// Converts a Switchboard Decimal to a fixed-point number with the target exponent
/// Note: Switchboard always uses scale 18 (value / 10^18)
fn decimal_to_fixed<Exp: Integer>(
  decimal: rust_decimal::Decimal,
) -> Result<UFix64<Exp>> {
  // Get the mantissa from the decimal
  let mantissa = decimal.mantissa();

  // Switchboard scale is always 18
  const SWITCHBOARD_SCALE: i32 = 18;

  // Our fixed point uses negative exponents (e.g., N8 = -8)
  let target_exp = Exp::to_i32();

  // Ensure mantissa is positive for unsigned fixed point
  if mantissa < 0 {
    return Err(SwitchboardOracleInvalidValue.into());
  }

  let mantissa_unsigned = mantissa.unsigned_abs();

  // If the scales match, we can use the mantissa directly
  if SWITCHBOARD_SCALE == -target_exp {
    let value_u64 = u64::try_from(mantissa_unsigned)
      .map_err(|_| SwitchboardOraclePriceRange)?;
    Ok(UFix64::new(value_u64))
  } else if SWITCHBOARD_SCALE > -target_exp {
    // Switchboard has more precision, need to divide
    let scale_diff = SWITCHBOARD_SCALE + target_exp;
    let divisor = 10u128.pow(scale_diff as u32);
    let scaled = mantissa_unsigned
      .checked_div(divisor)
      .ok_or(SwitchboardOraclePriceRange)?;
    let scaled_u64 =
      u64::try_from(scaled).map_err(|_| SwitchboardOraclePriceRange)?;
    Ok(UFix64::new(scaled_u64))
  } else {
    // Switchboard has less precision, need to multiply
    let scale_diff = -target_exp - SWITCHBOARD_SCALE;
    let multiplier = 10u128.pow(scale_diff as u32);
    let scaled = mantissa_unsigned
      .checked_mul(multiplier)
      .ok_or(SwitchboardOraclePriceRange)?;
    let scaled_u64 =
      u64::try_from(scaled).map_err(|_| SwitchboardOraclePriceRange)?;
    Ok(UFix64::new(scaled_u64))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use fix::typenum::N8;
  use rust_decimal::Decimal;

  #[test]
  fn test_decimal_to_fixed_same_scale() {
    // Switchboard uses scale=18
    // Value: 146.40110937 = 146401109370000000000 / 10^18
    // Target N8 = -8, so we need: value / 10^8 = 14640110937
    let decimal = Decimal::from_i128_with_scale(146401109370000000000, 18);
    let result = decimal_to_fixed::<N8>(decimal).unwrap();
    assert_eq!(result, UFix64::<N8>::new(14640110937));
  }

  #[test]
  fn test_decimal_to_fixed_higher_precision() {
    // Switchboard scale=18 (higher precision than N8=-8)
    // Value: 1.4640110937 = 1464011093700000000 / 10^18
    // With N8, this should be 146401109 / 10^8 (rounded down in division)
    let decimal = Decimal::from_i128_with_scale(1464011093700000000, 18);
    let result = decimal_to_fixed::<N8>(decimal).unwrap();
    assert_eq!(result, UFix64::<N8>::new(146401109));
  }

  #[test]
  fn test_decimal_to_fixed_lower_precision() {
    // This test doesn't make sense anymore since Switchboard scale is always 18
    // which is always higher than N8=-8
    // Test a simple case: 100.5 = 100500000000000000000 / 10^18
    // With N8, this should be 10050000000 / 10^8
    let decimal = Decimal::from_i128_with_scale(100500000000000000000, 18);
    let result = decimal_to_fixed::<N8>(decimal).unwrap();
    assert_eq!(result, UFix64::<N8>::new(10050000000));
  }

  #[test]
  fn test_decimal_to_fixed_negative() {
    // Test with negative mantissa (should fail)
    let decimal = Decimal::from_i128_with_scale(-14640110937, 8);
    let result = decimal_to_fixed::<N8>(decimal);
    assert!(result.is_err());
  }
}
