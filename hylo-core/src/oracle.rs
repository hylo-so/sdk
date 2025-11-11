//! Unified oracle interface supporting both Pyth and Switchboard oracles.
//!
//! This module provides a trait-based interface for querying prices from different
//! oracle providers. The `OraclePrice` trait can be implemented by any oracle type,
//! allowing for extensibility and testing with mock oracles.
//!
//! # Examples
//!
//! ```ignore
//! use hylo_core::oracle::{OraclePrice, OracleConfig};
//! use fix::typenum::N8;
//! use fix::prelude::*;
//!
//! // Configure oracle settings
//! let config = OracleConfig::new(
//!     60,  // 60 second staleness tolerance
//!     UFix64::<N8>::from_num(0.01),  // 1% confidence tolerance
//! );
//!
//! // Works with any oracle type that implements OraclePrice
//! let price = pyth_oracle.query_price(&clock, config)?;
//! let price = switchboard_quote.query_price(&clock, config)?;
//!
//! // Or pass as a generic
//! fn get_price<O: OraclePrice>(oracle: &O, clock: &impl SolanaClock) -> Result<PriceRange<N8>> {
//!     oracle.query_price(clock, config)
//! }
//! ```

use anchor_lang::prelude::Result;
use fix::prelude::*;
use fix::typenum::Integer;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use switchboard_on_demand::SwitchboardQuote;

use crate::solana_clock::SolanaClock;

// Re-export commonly used types for convenience
pub use crate::pyth::{query_pyth_price, OracleConfig, PriceRange};
pub use crate::switchboard::query_switchboard_price;

/// Trait for querying oracle prices.
///
/// This trait can be implemented for any oracle type, allowing for:
/// - Multiple oracle providers (Pyth, Switchboard, etc.)
/// - Mock oracles for testing
/// - Custom oracle implementations
pub trait OraclePrice {
  /// Query the current price from this oracle with validations.
  ///
  /// # Arguments
  /// * `clock` - Clock implementation for getting current slot/time
  /// * `config` - Oracle configuration with staleness interval and confidence tolerance
  ///
  /// # Returns
  /// A `PriceRange` with lower and upper bounds for the asset price
  fn query_price<Exp: Integer, C: SolanaClock>(
    &self,
    clock: &C,
    config: OracleConfig<Exp>,
  ) -> Result<PriceRange<Exp>>
  where
    UFix64<Exp>: FixExt;
}

/// Implementation of OraclePrice for Pyth's PriceUpdateV2
impl OraclePrice for PriceUpdateV2 {
  fn query_price<Exp: Integer, C: SolanaClock>(
    &self,
    clock: &C,
    config: OracleConfig<Exp>,
  ) -> Result<PriceRange<Exp>>
  where
    UFix64<Exp>: FixExt,
  {
    query_pyth_price(clock, self, config)
  }
}

/// Implementation of OraclePrice for Switchboard's SwitchboardQuote
impl OraclePrice for SwitchboardQuote {
  fn query_price<Exp: Integer, C: SolanaClock>(
    &self,
    clock: &C,
    config: OracleConfig<Exp>,
  ) -> Result<PriceRange<Exp>>
  where
    UFix64<Exp>: FixExt,
  {
    query_switchboard_price(clock, self, config)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::solana_clock::SolanaClock;
  use fix::typenum::N8;

  // Mock oracle for testing
  struct MockOracle {
    lower: u64,
    upper: u64,
  }

  impl OraclePrice for MockOracle {
    fn query_price<Exp: Integer, C: SolanaClock>(
      &self,
      _clock: &C,
      _config: OracleConfig<Exp>,
    ) -> Result<PriceRange<Exp>>
    where
      UFix64<Exp>: FixExt,
    {
      Ok(PriceRange {
        lower: UFix64::new(self.lower),
        upper: UFix64::new(self.upper),
      })
    }
  }

  #[test]
  fn test_trait_extensibility() {
    // Test that we can create custom oracle implementations
    struct TestClock;
    impl SolanaClock for TestClock {
      fn slot(&self) -> u64 {
        100
      }
      fn epoch(&self) -> u64 {
        10
      }
      fn epoch_start_timestamp(&self) -> i64 {
        0
      }
      fn leader_schedule_epoch(&self) -> u64 {
        10
      }
      fn unix_timestamp(&self) -> i64 {
        1000000
      }
    }

    let mock = MockOracle {
      lower: 10000000000, // $100 with 8 decimals
      upper: 10100000000, // $101 with 8 decimals
    };
    let config = OracleConfig::new(60, UFix64::<N8>::new(1000000)); // 1% tolerance
    let clock = TestClock;

    let result = mock.query_price(&clock, config);
    assert!(result.is_ok());
    let price_range = result.unwrap();
    assert_eq!(price_range.lower, UFix64::<N8>::new(10000000000));
    assert_eq!(price_range.upper, UFix64::<N8>::new(10100000000));
  }
}
