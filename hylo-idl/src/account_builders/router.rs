use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;

use crate::router::client::accounts::{
  InitializeExoRegistry, RegisterExoEntry, RouteV2,
};
use crate::{pda, router};

/// Builds the account context for initializing the canonical exo registry.
#[must_use]
pub fn initialize_exo_registry(admin: Pubkey) -> InitializeExoRegistry {
  InitializeExoRegistry {
    admin,
    hylo: pda::HYLO,
    exo_registry: pda::EXO_REGISTRY,
    system_program: system_program::ID,
  }
}

/// Builds the account context for registering an exo pair with the router.
#[must_use]
pub fn register_exo_entry(
  admin: Pubkey,
  collateral_mint: Pubkey,
) -> RegisterExoEntry {
  RegisterExoEntry {
    admin,
    hylo: pda::HYLO,
    exo_registry: pda::EXO_REGISTRY,
    collateral_mint,
    levercoin_mint: pda::exo_levercoin_mint(collateral_mint),
    exo_pair: pda::exo_pair(collateral_mint),
    event_authority: pda::ROUTER_EVENT_AUTHORITY,
    program: router::ID,
  }
}

/// Builds the fixed account context for `route_v2`.
#[must_use]
pub const fn route_v2() -> RouteV2 {
  RouteV2 {
    exo_registry: pda::EXO_REGISTRY,
  }
}
