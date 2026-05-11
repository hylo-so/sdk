#![allow(clippy::pub_underscore_fields, clippy::too_many_arguments)]

extern crate anchor_lang;

mod codegen {
  anchor_lang::declare_program!(hylo_earn_pool);
  anchor_lang::declare_program!(hylo_exchange);
  anchor_lang::declare_program!(hylo_router);
}

mod account_builders;
mod instruction_builders;

pub mod exchange {
  #[cfg(feature = "shadow")]
  use anchor_lang::prelude::{pubkey, Pubkey};

  pub use super::account_builders::exchange as account_builders;
  pub use super::codegen::hylo_exchange::*;
  pub use super::instruction_builders::exchange as instruction_builders;

  #[cfg(feature = "shadow")]
  pub const ID: Pubkey =
    pubkey!("HYSheX1FkQgYvzUsyPEuzXrGp2tNAWMvbuNVFETXGAXH");
}

pub mod earn_pool {
  #[cfg(feature = "shadow")]
  use anchor_lang::prelude::{pubkey, Pubkey};

  pub use super::account_builders::earn_pool as account_builders;
  pub use super::codegen::hylo_earn_pool::*;
  pub use super::instruction_builders::earn_pool as instruction_builders;

  #[cfg(feature = "shadow")]
  pub const ID: Pubkey =
    pubkey!("HYShspCfhpuFXJKUBunV7evNyJsGuq6M9qBUm6PPA8Xk");
}

pub mod router {
  #[cfg(feature = "shadow")]
  use anchor_lang::prelude::{pubkey, Pubkey};

  pub use super::codegen::hylo_router::*;
  pub use super::instruction_builders::router as instruction_builders;

  #[cfg(feature = "shadow")]
  pub const ID: Pubkey = pubkey!("hyshRoSsynsCxF5Dt9KnHc5pS1u8saVT79NywtWUSsv");
}

pub mod pda;
pub mod tokens;
pub mod type_bridge;
