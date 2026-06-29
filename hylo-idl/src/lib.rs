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

pub mod pda;
pub mod tokens;
pub mod type_bridge;

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

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::{pubkey, Pubkey};
  use anchor_lang::Id;

  use crate::{earn_pool, exchange, router};

  #[cfg(not(feature = "shadow"))]
  mod expected {
    use super::*;
    pub const EARN_POOL: Pubkey =
      pubkey!("HYeARNuP78WqakviNj2hmLK935yL8ENfSyGtPk3UgB3r");
    pub const EXCHANGE: Pubkey =
      pubkey!("HYEXCHtHkBagdStcJCp3xbbb9B7sdMdWXFNj6mdsG4hn");
    pub const ROUTER: Pubkey =
      pubkey!("hyRouTRDAgn65xyyJ3L5c4k5SFmSdr3NxDV8Euzjy3f");
  }

  #[cfg(feature = "shadow")]
  mod expected {
    use super::*;
    pub const EARN_POOL: Pubkey =
      pubkey!("HYShEAST5PHe5EFxUPYUgzXsmSo88VVdDqJE21jXBQ7N");
    pub const EXCHANGE: Pubkey =
      pubkey!("hyshEX5sNEYhnYPMm8MwMThhBRPuLN3rjoYDbC9esPQ");
    pub const ROUTER: Pubkey =
      pubkey!("HyshRo2hkqXGcyCfKU22zhSBPMwokmAnEoxDGeVQz7d");
  }

  #[test]
  fn module_ids_match_expected() {
    assert_eq!(earn_pool::ID, expected::EARN_POOL);
    assert_eq!(exchange::ID, expected::EXCHANGE);
    assert_eq!(router::ID, expected::ROUTER);
  }

  #[test]
  fn id_trait_impls_match_expected() {
    assert_eq!(earn_pool::program::HyloEarnPool::id(), expected::EARN_POOL);
    assert_eq!(exchange::program::HyloExchange::id(), expected::EXCHANGE);
    assert_eq!(router::program::HyloRouter::id(), expected::ROUTER);
  }
}
