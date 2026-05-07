use crate::borrow_rate::BorrowRateConfig;
use crate::fees::controller::{FeePair, LevercoinFees, StablecoinFees};
use crate::lst::sol_price::LstSolPrice;
use crate::lst::total_sol_cache::TotalSolCache;
use crate::rebalance::pricing::RebalanceCurveConfig;
use crate::slippage_config::SlippageConfig;
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
    LevercoinFees::new(
      idl.normal.into(),
      idl.sell_zone_1.into(),
      idl.sell_zone_2.into(),
    )
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

impl From<hylo_idl::exchange::types::BorrowRateConfig> for BorrowRateConfig {
  fn from(
    idl: hylo_idl::exchange::types::BorrowRateConfig,
  ) -> BorrowRateConfig {
    BorrowRateConfig::new(idl.rate.into(), idl.fee.into())
  }
}

impl From<hylo_idl::exchange::types::RebalanceCurveConfig>
  for RebalanceCurveConfig
{
  fn from(
    idl: hylo_idl::exchange::types::RebalanceCurveConfig,
  ) -> RebalanceCurveConfig {
    RebalanceCurveConfig::new(idl.floor_mult.into(), idl.ceil_mult.into())
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

impl From<SlippageConfig> for hylo_idl::router::types::SlippageConfig {
  fn from(val: SlippageConfig) -> Self {
    let exchange_sc: hylo_idl::exchange::types::SlippageConfig = val.into();
    exchange_sc.into()
  }
}

impl From<FeePair> for hylo_idl::exchange::types::FeePair {
  fn from(val: FeePair) -> Self {
    hylo_idl::exchange::types::FeePair {
      mint: val.mint.into(),
      redeem: val.redeem.into(),
    }
  }
}

impl From<StablecoinFees> for hylo_idl::exchange::types::StablecoinFees {
  fn from(val: StablecoinFees) -> Self {
    hylo_idl::exchange::types::StablecoinFees {
      normal: val.normal.into(),
      mode_1: val.mode_1.into(),
    }
  }
}

impl From<LevercoinFees> for hylo_idl::exchange::types::LevercoinFees {
  fn from(val: LevercoinFees) -> Self {
    hylo_idl::exchange::types::LevercoinFees {
      normal: val.normal.into(),
      sell_zone_1: val.sell_zone_1.into(),
      sell_zone_2: val.sell_zone_2.into(),
    }
  }
}

impl From<YieldHarvestConfig>
  for hylo_idl::exchange::types::YieldHarvestConfig
{
  fn from(val: YieldHarvestConfig) -> Self {
    hylo_idl::exchange::types::YieldHarvestConfig {
      allocation: val.allocation.into(),
      fee: val.fee.into(),
    }
  }
}

impl From<RebalanceCurveConfig>
  for hylo_idl::exchange::types::RebalanceCurveConfig
{
  fn from(val: RebalanceCurveConfig) -> Self {
    hylo_idl::exchange::types::RebalanceCurveConfig {
      floor_mult: val.floor_mult.into(),
      ceil_mult: val.ceil_mult.into(),
    }
  }
}

impl From<BorrowRateConfig> for hylo_idl::exchange::types::BorrowRateConfig {
  fn from(val: BorrowRateConfig) -> Self {
    hylo_idl::exchange::types::BorrowRateConfig {
      rate: val.rate.into(),
      fee: val.fee.into(),
    }
  }
}
