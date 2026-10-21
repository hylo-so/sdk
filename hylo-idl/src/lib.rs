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
  anchor_lang::declare_program!(hylo_oracle);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_earn_pool_shadow);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_exchange_shadow);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_router_shadow);
  #[cfg(feature = "shadow")]
  anchor_lang::declare_program!(hylo_oracle_shadow);
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

pub mod oracle {
  pub use super::account_builders::oracle as account_builders;
  #[cfg(not(feature = "shadow"))]
  pub use super::codegen::hylo_oracle::*;
  #[cfg(feature = "shadow")]
  pub use super::codegen::hylo_oracle_shadow::*;
  pub use super::instruction_builders::oracle as instruction_builders;
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::{pubkey, Pubkey};
  use anchor_lang::{Discriminator, Id, ToAccountMetas};
  use const_crypto::sha2::Sha256;

  use crate::{earn_pool, exchange, oracle, pda, router};

  #[cfg(not(feature = "shadow"))]
  mod expected {
    use super::*;
    pub const EARN_POOL: Pubkey =
      pubkey!("HysTabVUfmQBFcmzu1ctRd1Y1fxd66RBpboy1bmtDSQQ");
    pub const EXCHANGE: Pubkey =
      pubkey!("HYEXCHtHkBagdStcJCp3xbbb9B7sdMdWXFNj6mdsG4hn");
    pub const ROUTER: Pubkey =
      pubkey!("hyRouTRDAgn65xyyJ3L5c4k5SFmSdr3NxDV8Euzjy3f");
    pub const ORACLE: Pubkey =
      pubkey!("hyorViHghDYU4Yd15DZFHDQXJBtzXqVHuypcvN3Gj7t");
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
    pub const ORACLE: Pubkey =
      pubkey!("HySHy1kwAfZbPAfB3zutXgrBXsXQHdtqA1GJu24W4b9F");
  }

  #[test]
  fn module_ids_match_expected() {
    assert_eq!(earn_pool::ID, expected::EARN_POOL);
    assert_eq!(exchange::ID, expected::EXCHANGE);
    assert_eq!(router::ID, expected::ROUTER);
    assert_eq!(oracle::ID, expected::ORACLE);
  }

  #[test]
  fn id_trait_impls_match_expected() {
    assert_eq!(earn_pool::program::HyloEarnPool::id(), expected::EARN_POOL);
    assert_eq!(exchange::program::HyloExchange::id(), expected::EXCHANGE);
    assert_eq!(router::program::HyloRouter::id(), expected::ROUTER);
    assert_eq!(oracle::program::HyloOracle::id(), expected::ORACLE);
  }

  #[test]
  fn router_registry_derivations_match_program_seeds() {
    let expected_registry = Pubkey::find_program_address(
      &[&router::constants::EXO_REGISTRY],
      &router::ID,
    )
    .0;

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

  /// Asserts `route` and `route_v2` keep the default Anchor discriminators
  /// for their names and `route` has no fixed accounts.
  #[test]
  fn route_instructions_match_anchor_defaults() {
    let route = Sha256::new().update(b"global:route").finalize();
    let route_v2 = Sha256::new().update(b"global:route_v2").finalize();
    assert_eq!(router::client::args::Route::DISCRIMINATOR, &route[..8]);
    assert!(router::client::accounts::Route {}
      .to_account_metas(None)
      .is_empty());
    assert_eq!(router::client::args::RouteV2::DISCRIMINATOR, &route_v2[..8]);
  }

  #[test]
  fn oracle_price_aggregate_matches_program_seeds() {
    let id: u16 = 3;
    let (expected_aggregate, _bump) = Pubkey::find_program_address(
      &[&oracle::constants::PRICE_AGGREGATE, &id.to_le_bytes()],
      &oracle::ID,
    );
    assert_eq!(pda::price_aggregate(id), expected_aggregate);
    assert_eq!(pda::ORACLE_EVENT_AUTHORITY, pda::event_auth(oracle::ID));

    let payer = Pubkey::new_unique();
    let initialize =
      oracle::instruction_builders::initialize_price_aggregate(payer, id);
    assert_eq!(initialize.program_id, oracle::ID);
    assert!(initialize.accounts.iter().any(|account| {
      account.pubkey == expected_aggregate && account.is_writable
    }));

    let treasury = Pubkey::new_unique();
    let args = oracle::client::args::UpdateWithPythLazer {
      channel_id: 4,
      message_data: vec![1, 2, 3],
      ed25519_instruction_index: 0,
      signature_index: 0,
    };
    let update = oracle::instruction_builders::update_with_pyth_lazer(
      payer,
      treasury,
      &[id],
      &args,
    );
    assert_eq!(update.program_id, oracle::ID);
    assert!(update
      .accounts
      .iter()
      .any(|account| { account.pubkey == treasury && account.is_writable }));
    assert!(update.accounts.iter().any(|account| {
      account.pubkey == expected_aggregate && account.is_writable
    }));
  }
}
