#![allow(clippy::pub_underscore_fields, clippy::too_many_arguments)]

extern crate anchor_lang;

mod codegen {
  #[cfg(not(feature = "shadow"))]
  anchor_lang::declare_program!(hylo_earn_pool);
  #[cfg(not(feature = "shadow"))]
  anchor_lang::declare_program!(hylo_exchange);
  #[cfg(not(feature = "shadow"))]
  anchor_lang::declare_program!(hylo_router);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_earn_pool_shadow);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_exchange_shadow);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_router_shadow);
}

mod account_builders;
mod instruction_builders;

pub mod exchange {
  pub use super::account_builders::exchange as account_builders;
  #[cfg(not(feature = "shadow"))]
  pub use super::codegen::hylo_exchange::*;
  #[cfg(feature = "shadow")]
  pub use super::codegen::hylo_exchange_shadow::*;
  pub use super::instruction_builders::exchange as instruction_builders;
}

pub mod earn_pool {
  pub use super::account_builders::earn_pool as account_builders;
  #[cfg(not(feature = "shadow"))]
  pub use super::codegen::hylo_earn_pool::*;
  #[cfg(feature = "shadow")]
  pub use super::codegen::hylo_earn_pool_shadow::*;
  pub use super::instruction_builders::earn_pool as instruction_builders;
}

pub mod router {
  #[cfg(not(feature = "shadow"))]
  pub use super::codegen::hylo_router::*;
  #[cfg(feature = "shadow")]
  pub use super::codegen::hylo_router_shadow::*;
  pub use super::instruction_builders::router as instruction_builders;
}

pub mod pda;
pub mod tokens;
pub mod type_bridge;

#[cfg(test)]
mod program_id_tests {
  use anchor_lang::prelude::{pubkey, Pubkey};
  use anchor_lang::Id;

  use crate::{earn_pool, exchange, router};

  #[cfg(not(feature = "shadow"))]
  const EARN_POOL: Pubkey =
    pubkey!("HysTabVUfmQBFcmzu1ctRd1Y1fxd66RBpboy1bmtDSQQ");
  #[cfg(not(feature = "shadow"))]
  const EXCHANGE: Pubkey =
    pubkey!("HYEXCHtHkBagdStcJCp3xbbb9B7sdMdWXFNj6mdsG4hn");
  #[cfg(not(feature = "shadow"))]
  const ROUTER: Pubkey = pubkey!("hyRouTRDAgn65xyyJ3L5c4k5SFmSdr3NxDV8Euzjy3f");

  #[cfg(feature = "shadow")]
  const EARN_POOL: Pubkey =
    pubkey!("HYShspCfhpuFXJKUBunV7evNyJsGuq6M9qBUm6PPA8Xk");
  #[cfg(feature = "shadow")]
  const EXCHANGE: Pubkey =
    pubkey!("HYSheX1FkQgYvzUsyPEuzXrGp2tNAWMvbuNVFETXGAXH");
  #[cfg(feature = "shadow")]
  const ROUTER: Pubkey = pubkey!("hyshRoSsynsCxF5Dt9KnHc5pS1u8saVT79NywtWUSsv");

  #[test]
  fn module_ids_match_expected() {
    assert_eq!(earn_pool::ID, EARN_POOL);
    assert_eq!(exchange::ID, EXCHANGE);
    assert_eq!(router::ID, ROUTER);
  }

  #[test]
  fn id_trait_impls_match_expected() {
    assert_eq!(earn_pool::program::HyloEarnPool::id(), EARN_POOL);
    assert_eq!(exchange::program::HyloExchange::id(), EXCHANGE);
    assert_eq!(router::program::HyloRouter::id(), ROUTER);
  }
}
