use crate::fee_controller::{FeePair, LevercoinFees, StablecoinFees};
use crate::lst_sol_price::LstSolPrice;
use crate::total_sol_cache::TotalSolCache;
use fix::prelude::UFixValue64;

pub fn convert_ufixvalue64(
  idl: hylo_idl::hylo_exchange::types::UFixValue64,
) -> UFixValue64 {
  UFixValue64 {
    bits: idl.bits,
    exp: idl.exp,
  }
}

impl From<hylo_idl::hylo_exchange::types::LstSolPrice> for LstSolPrice {
  fn from(idl: hylo_idl::hylo_exchange::types::LstSolPrice) -> Self {
    LstSolPrice::new(convert_ufixvalue64(idl.price), idl.epoch)
  }
}

impl From<hylo_idl::hylo_exchange::types::StablecoinFees> for StablecoinFees {
  fn from(idl: hylo_idl::hylo_exchange::types::StablecoinFees) -> Self {
    StablecoinFees::new(idl.normal.into(), idl.mode_1.into())
  }
}

impl From<hylo_idl::hylo_exchange::types::LevercoinFees> for LevercoinFees {
  fn from(idl: hylo_idl::hylo_exchange::types::LevercoinFees) -> Self {
    LevercoinFees::new(idl.normal.into(), idl.mode_1.into(), idl.mode_2.into())
  }
}

impl From<hylo_idl::hylo_exchange::types::FeePair> for FeePair {
  fn from(idl: hylo_idl::hylo_exchange::types::FeePair) -> FeePair {
    FeePair::new(
      convert_ufixvalue64(idl.mint),
      convert_ufixvalue64(idl.redeem),
    )
  }
}

impl From<hylo_idl::hylo_exchange::types::TotalSolCache> for TotalSolCache {
  fn from(idl: hylo_idl::hylo_exchange::types::TotalSolCache) -> TotalSolCache {
    TotalSolCache {
      current_update_epoch: idl.current_update_epoch,
      total_sol: convert_ufixvalue64(idl.total_sol),
    }
  }
}
