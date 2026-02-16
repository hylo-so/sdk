use crate::fee_controller::{FeePair, LevercoinFees, StablecoinFees};
use crate::lst_sol_price::LstSolPrice;
use crate::slippage_config::SlippageConfig;
use crate::total_sol_cache::TotalSolCache;
use crate::virtual_stablecoin::VirtualStablecoin;
use crate::yields::{HarvestCache, YieldHarvestConfig};

impl From<hylo_idl::exchange::types::LstSolPrice> for LstSolPrice {
  fn from(idl: hylo_idl::exchange::types::LstSolPrice) -> Self {
    LstSolPrice::new(idl.price.into(), idl.epoch)
  }
}

impl From<hylo_idl::exchange::types::StablecoinFees> for StablecoinFees {
  fn from(idl: hylo_idl::exchange::types::StablecoinFees) -> StablecoinFees {
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
    FeePair::new(idl.mint.into(), idl.redeem.into())
  }
}

impl From<hylo_idl::exchange::types::TotalSolCache> for TotalSolCache {
  fn from(idl: hylo_idl::exchange::types::TotalSolCache) -> TotalSolCache {
    TotalSolCache {
      current_update_epoch: idl.current_update_epoch,
      total_sol: idl.total_sol.into(),
    }
  }
}

impl From<hylo_idl::exchange::types::YieldHarvestConfig>
  for YieldHarvestConfig
{
  fn from(idl: hylo_idl::exchange::types::YieldHarvestConfig) -> Self {
    YieldHarvestConfig {
      allocation: idl.allocation.into(),
      fee: idl.fee.into(),
    }
  }
}

impl From<hylo_idl::exchange::types::HarvestCache> for HarvestCache {
  fn from(idl: hylo_idl::exchange::types::HarvestCache) -> Self {
    HarvestCache {
      epoch: idl.epoch,
      stability_pool_cap: idl.stability_pool_cap.into(),
      stablecoin_to_pool: idl.stablecoin_to_pool.into(),
    }
  }
}

impl From<hylo_idl::exchange::types::VirtualStablecoin> for VirtualStablecoin {
  fn from(
    idl: hylo_idl::exchange::types::VirtualStablecoin,
  ) -> VirtualStablecoin {
    VirtualStablecoin {
      supply: idl.supply.into(),
    }
  }
}

impl From<SlippageConfig> for hylo_idl::exchange::types::SlippageConfig {
  fn from(val: SlippageConfig) -> Self {
    hylo_idl::exchange::types::SlippageConfig {
      expected_token_out: val.expected_token_out.into(),
      slippage_tolerance: val.slippage_tolerance.into(),
    }
  }
}
