#![allow(clippy::pub_underscore_fields)]

extern crate anchor_lang;

anchor_lang::declare_program!(hylo_exchange);
anchor_lang::declare_program!(hylo_stability_pool);

pub mod instructions;
pub mod pda;
pub mod tokens;
