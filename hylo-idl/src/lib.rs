#![allow(clippy::pub_underscore_fields, clippy::too_many_arguments)]

extern crate anchor_lang;

mod codegen {
  #[cfg(not(feature = "shadow"))]
  anchor_lang::declare_program!(hylo_earn_pool);
  #[cfg(not(feature = "shadow"))]
  anchor_lang::declare_program!(hylo_exchange);
  #[cfg(not(feature = "shadow"))]
  anchor_lang::declare_program!(hylo_router);
  #[cfg(not(feature = "shadow"))]
  anchor_lang::declare_program!(hylo_trigger_orders);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_earn_pool_shadow);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_exchange_shadow);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_router_shadow);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_trigger_orders_shadow);
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

pub mod trigger_orders {
  pub use super::account_builders::trigger_orders as account_builders;
  // `instruction_builders` module added in later tasks.
  #[cfg(not(feature = "shadow"))]
  pub use super::codegen::hylo_trigger_orders::*;
  #[cfg(feature = "shadow")]
  pub use super::codegen::hylo_trigger_orders_shadow::*;
}

mod trigger_orders_ext;

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::{pubkey, Pubkey};
  use anchor_lang::Id;

  use crate::{earn_pool, exchange, router, trigger_orders};

  #[cfg(not(feature = "shadow"))]
  mod expected {
    use super::*;
    pub const EARN_POOL: Pubkey =
      pubkey!("HysTabVUfmQBFcmzu1ctRd1Y1fxd66RBpboy1bmtDSQQ");
    pub const EXCHANGE: Pubkey =
      pubkey!("HYEXCHtHkBagdStcJCp3xbbb9B7sdMdWXFNj6mdsG4hn");
    pub const ROUTER: Pubkey =
      pubkey!("hyRouTRDAgn65xyyJ3L5c4k5SFmSdr3NxDV8Euzjy3f");
    pub const TRIGGER_ORDERS: Pubkey =
      pubkey!("2GbKbBumaPjHsJ5qzkUt6cnsyTBT6aPNj3gq2mvocpFW");
  }

  #[cfg(feature = "shadow")]
  mod expected {
    use super::*;
    pub const EARN_POOL: Pubkey =
      pubkey!("HYShspCfhpuFXJKUBunV7evNyJsGuq6M9qBUm6PPA8Xk");
    pub const EXCHANGE: Pubkey =
      pubkey!("HYSheX1FkQgYvzUsyPEuzXrGp2tNAWMvbuNVFETXGAXH");
    pub const ROUTER: Pubkey =
      pubkey!("hyshRoSsynsCxF5Dt9KnHc5pS1u8saVT79NywtWUSsv");
    pub const TRIGGER_ORDERS: Pubkey =
      pubkey!("4tgWjqeTdwjCrvHStnR5dzW8WGADRyAy7jkS32zNnJuM");
  }

  #[test]
  fn module_ids_match_expected() {
    assert_eq!(earn_pool::ID, expected::EARN_POOL);
    assert_eq!(exchange::ID, expected::EXCHANGE);
    assert_eq!(router::ID, expected::ROUTER);
    assert_eq!(trigger_orders::ID, expected::TRIGGER_ORDERS);
  }

  #[test]
  fn id_trait_impls_match_expected() {
    assert_eq!(earn_pool::program::HyloEarnPool::id(), expected::EARN_POOL);
    assert_eq!(exchange::program::HyloExchange::id(), expected::EXCHANGE);
    assert_eq!(router::program::HyloRouter::id(), expected::ROUTER);
  }
}

#[cfg(test)]
mod trigger_orders_smoke_tests {
  // Confirm the generated events module is reachable. Task 17 depends on
  // this path resolving; an unresolved path here is a hard compile error.
  // If the macro generates events under a different sub-module name,
  // adjust `pub mod trigger_orders { ... }` in lib.rs accordingly.
  #[allow(unused_imports)]
  use crate::trigger_orders::events::{
    TriggerOrderCancelled, TriggerOrderCreated, TriggerOrderFilled,
  };

  #[test]
  fn tip_constant_resolves() {
    let tip: u64 = crate::trigger_orders::constants::EXECUTOR_TIP_LAMPORTS;
    assert_eq!(tip, 5_000_000);
  }

  /// PDA derived locally must equal the value recorded on the deployed
  /// shadow program. Locks the seed + program-ID pair against silent
  /// regressions in either repo.
  #[cfg(feature = "shadow")]
  #[test]
  fn shadow_event_authority_matches_deployed() {
    use anchor_lang::prelude::Pubkey;

    let (pda, _bump) = Pubkey::find_program_address(
      &[b"__event_authority"],
      &crate::trigger_orders::ID,
    );
    let recorded: Pubkey = "32Sig6DM39gxcVqDWM666ts3auNwjRpXuazUEhyqWP63"
      .parse()
      .unwrap();
    assert_eq!(pda, recorded);
  }
}
