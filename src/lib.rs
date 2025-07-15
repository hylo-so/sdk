#![allow(clippy::pub_underscore_fields)]

extern crate anchor_lang;

anchor_lang::declare_program!(hylo_exchange);
anchor_lang::declare_program!(hylo_stability_pool);

mod exchange_client;
pub mod pda;
mod stability_pool_client;
pub mod util;

pub mod exchange {
  pub use super::exchange_client::*;
  pub use super::hylo_exchange::*;
}

pub mod stability_pool {
  pub use super::hylo_stability_pool::*;
  pub use super::stability_pool_client::*;
}
