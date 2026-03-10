#![allow(clippy::pub_underscore_fields, clippy::too_many_arguments)]

extern crate anchor_lang;

mod codegen {
  anchor_lang::declare_program!(hylo_exchange);
  anchor_lang::declare_program!(hylo_stability_pool);
}

mod account_builders;
mod instruction_builders;

pub mod exchange {
  pub use super::account_builders::exchange as account_builders;
  pub use super::codegen::hylo_exchange::*;
  pub use super::instruction_builders::exchange as instruction_builders;
}

pub mod stability_pool {
  pub use super::account_builders::stability_pool as account_builders;
  pub use super::codegen::hylo_stability_pool::*;
  pub use super::instruction_builders::stability_pool as instruction_builders;
}

pub mod pda;
pub mod tokens;
pub mod type_bridge;
