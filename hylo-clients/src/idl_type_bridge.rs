use fix::prelude::UFixValue64;
use hylo_core::fee_controller::{FeePair, LevercoinFees, StablecoinFees};
use hylo_core::lst_sol_price::LstSolPrice;
use hylo_core::total_sol_cache::TotalSolCache;

impl From<crate::hylo_exchange::types::LstSolPrice> for LstSolPrice {
  fn from(idl: crate::hylo_exchange::types::LstSolPrice) -> Self {
    LstSolPrice::new(idl.price.into(), idl.epoch)
  }
}

impl From<crate::hylo_exchange::types::StablecoinFees> for StablecoinFees {
  fn from(idl: crate::hylo_exchange::types::StablecoinFees) -> Self {
    StablecoinFees::new(idl.normal.into(), idl.mode_1.into())
  }
}

impl From<crate::hylo_exchange::types::LevercoinFees> for LevercoinFees {
  fn from(idl: crate::hylo_exchange::types::LevercoinFees) -> Self {
    LevercoinFees::new(idl.normal.into(), idl.mode_1.into(), idl.mode_2.into())
  }
}

impl From<crate::hylo_exchange::types::FeePair> for FeePair {
  fn from(idl: crate::hylo_exchange::types::FeePair) -> FeePair {
    FeePair::new(idl.mint.into(), idl.redeem.into())
  }
}

impl From<crate::hylo_exchange::types::UFixValue64> for UFixValue64 {
  fn from(idl: crate::hylo_exchange::types::UFixValue64) -> Self {
    UFixValue64 {
      bits: idl.bits,
      exp: idl.exp,
    }
  }
}

impl From<crate::hylo_exchange::types::TotalSolCache> for TotalSolCache {
  fn from(idl: crate::hylo_exchange::types::TotalSolCache) -> TotalSolCache {
    TotalSolCache {
      current_update_epoch: idl.current_update_epoch,
      total_sol: idl.total_sol.into(),
    }
  }
}
