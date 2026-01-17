use fix::prelude::UFixValue64;

use crate::fee_controller::{FeePair, LevercoinFees, StablecoinFees};
use crate::lst_sol_price::LstSolPrice;
use crate::lst_swap_config::LstSwapConfig;
use crate::slippage_config::SlippageConfig;
use crate::total_sol_cache::TotalSolCache;
use crate::yields::{YieldHarvestCache, YieldHarvestConfig};

#[must_use]
pub fn convert_ufixvalue64(
  idl: hylo_idl::exchange::types::UFixValue64,
) -> UFixValue64 {
  UFixValue64 {
    bits: idl.bits,
    exp: idl.exp,
  }
}

#[must_use]
pub fn reconvert_ufixvalue64(
  val: UFixValue64,
) -> hylo_idl::exchange::types::UFixValue64 {
  hylo_idl::exchange::types::UFixValue64 {
    bits: val.bits,
    exp: val.exp,
  }
}

impl From<hylo_idl::exchange::types::LstSolPrice> for LstSolPrice {
  fn from(idl: hylo_idl::exchange::types::LstSolPrice) -> Self {
    LstSolPrice::new(convert_ufixvalue64(idl.price), idl.epoch)
  }
}

impl From<hylo_idl::exchange::types::StablecoinFees> for StablecoinFees {
  fn from(idl: hylo_idl::exchange::types::StablecoinFees) -> Self {
    StablecoinFees::new(idl.normal.into(), idl.mode_1.into())
  }
}

impl From<hylo_idl::exchange::types::LevercoinFees> for LevercoinFees {
  fn from(idl: hylo_idl::exchange::types::LevercoinFees) -> Self {
    LevercoinFees::new(idl.normal.into(), idl.mode_1.into(), idl.mode_2.into())
  }
}

impl From<hylo_idl::exchange::types::FeePair> for FeePair {
  fn from(idl: hylo_idl::exchange::types::FeePair) -> FeePair {
    FeePair::new(
      convert_ufixvalue64(idl.mint),
      convert_ufixvalue64(idl.redeem),
    )
  }
}

impl From<hylo_idl::exchange::types::TotalSolCache> for TotalSolCache {
  fn from(idl: hylo_idl::exchange::types::TotalSolCache) -> TotalSolCache {
    TotalSolCache {
      current_update_epoch: idl.current_update_epoch,
      total_sol: convert_ufixvalue64(idl.total_sol),
    }
  }
}

impl From<hylo_idl::exchange::types::YieldHarvestConfig>
  for YieldHarvestConfig
{
  fn from(idl: hylo_idl::exchange::types::YieldHarvestConfig) -> Self {
    YieldHarvestConfig {
      allocation: convert_ufixvalue64(idl.allocation),
      fee: convert_ufixvalue64(idl.fee),
    }
  }
}

impl From<hylo_idl::exchange::types::YieldHarvestCache> for YieldHarvestCache {
  fn from(idl: hylo_idl::exchange::types::YieldHarvestCache) -> Self {
    YieldHarvestCache {
      epoch: idl.epoch,
      stability_pool_cap: convert_ufixvalue64(idl.stability_pool_cap),
      stablecoin_yield_to_pool: convert_ufixvalue64(
        idl.stablecoin_yield_to_pool,
      ),
    }
  }
}

impl From<SlippageConfig> for hylo_idl::exchange::types::SlippageConfig {
  fn from(val: SlippageConfig) -> Self {
    hylo_idl::exchange::types::SlippageConfig {
      expected_token_out: reconvert_ufixvalue64(val.expected_token_out),
      slippage_tolerance: reconvert_ufixvalue64(val.slippage_tolerance),
    }
  }
}

impl From<hylo_idl::exchange::types::LstSwapConfig> for LstSwapConfig {
  fn from(idl: hylo_idl::exchange::types::LstSwapConfig) -> Self {
    LstSwapConfig {
      fee: convert_ufixvalue64(idl.fee),
    }
  }
}
