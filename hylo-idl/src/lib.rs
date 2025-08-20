#![allow(clippy::pub_underscore_fields)]

extern crate anchor_lang;

anchor_lang::declare_program!(hylo_exchange);
anchor_lang::declare_program!(hylo_stability_pool);

pub mod pda;

pub mod exchange {
  pub use super::hylo_exchange::*;
}

pub mod stability_pool {
  pub use super::hylo_stability_pool::*;
}
