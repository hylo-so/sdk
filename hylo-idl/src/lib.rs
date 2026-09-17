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
  pub use super::account_builders::router as account_builders;
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

  use crate::{earn_pool, exchange, pda, router};

  #[cfg(not(feature = "shadow"))]
  mod expected {
    use super::*;
    pub const EARN_POOL: Pubkey =
      pubkey!("HysTabVUfmQBFcmzu1ctRd1Y1fxd66RBpboy1bmtDSQQ");
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

  #[test]
  fn router_registry_derivations_match_program_seeds() {
    let expected_registry = Pubkey::find_program_address(
      &[&router::constants::EXO_REGISTRY],
      &router::ID,
    )
    .0;

    assert_eq!(pda::exo_registry(), expected_registry);
    assert_eq!(pda::EXO_REGISTRY, expected_registry);
    assert_eq!(pda::ROUTER_EVENT_AUTHORITY, pda::event_auth(router::ID));
  }

  #[test]
  fn router_exo_registry_builders_use_canonical_accounts() {
    let admin = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();

    let initialize =
      router::instruction_builders::initialize_exo_registry(admin);
    assert_eq!(initialize.program_id, router::ID);
    assert!(initialize.accounts.iter().any(|account| account.pubkey
      == pda::EXO_REGISTRY
      && account.is_writable));

    let register =
      router::instruction_builders::register_exo_entry(admin, collateral_mint);
    assert_eq!(register.program_id, router::ID);
    assert!(register.accounts.iter().any(|account| account.pubkey
      == pda::EXO_REGISTRY
      && account.is_writable));
    assert!(register.accounts.iter().any(|account| {
      account.pubkey == pda::exo_levercoin_mint(collateral_mint)
    }));
  }
}
