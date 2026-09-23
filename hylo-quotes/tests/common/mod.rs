//! Shared fixtures for snapshot-based tests.

#![allow(dead_code)]

use std::fs::File;

use anchor_lang::solana_program::clock::Clock;
use anyhow::{anyhow, Result};
use fix::prelude::*;
use hylo_core::exchange_context::ExchangeContext;
use hylo_quotes::prelude::{ProtocolAccounts, ProtocolState};
use hylo_quotes::protocol_state::ExoPairState;
use serde_json::from_reader;

/// CR inside the redeem fee-curve domain (stops at 1.50).
pub const CR_IN_DOMAIN: UFix64<N9> = UFix64::constant(1_400_000_000);

/// CR above the redeem fee-curve domain.
pub const CR_ABOVE_DOMAIN: UFix64<N9> = UFix64::constant(1_600_000_000);

/// Loads the mainnet snapshot into protocol state.
///
/// # Errors
/// * File IO, JSON, or state construction
pub fn load_state() -> Result<ProtocolState<Clock>> {
  let path = format!(
    "{}/tests/data/protocol-state-1039-295160.json",
    env!("CARGO_MANIFEST_DIR")
  );
  let file = File::open(path)?;
  let accounts = from_reader::<_, ProtocolAccounts>(file)?;
  ProtocolState::try_from(&accounts)
}

/// `cr * supply / usd_lower`: collateral that puts a pair at `cr`.
fn collateral_for_cr(
  supply: UFix64<N6>,
  usd_lower: UFix64<N9>,
  cr: UFix64<N9>,
) -> Result<UFix64<N9>> {
  supply
    .convert::<N9>()
    .mul_div_floor(cr, usd_lower)
    .ok_or_else(|| anyhow!("collateral for target CR overflows"))
}

/// Rewrites total SOL so the LST pair projects to `target_cr`.
///
/// # Errors
/// * Supply read or arithmetic overflow
pub fn with_lst_cr(
  mut state: ProtocolState<Clock>,
  target_cr: UFix64<N9>,
) -> Result<ProtocolState<Clock>> {
  let supply = state.exchange_context.virtual_stablecoin_supply()?;
  let lower = state.exchange_context.sol_usd_price.lower;
  state.exchange_context.total_sol =
    collateral_for_cr(supply, lower, target_cr)?;
  Ok(state)
}

/// Rewrites an exo pair's collateral so it projects to `target_cr`.
///
/// # Errors
/// * Supply read or arithmetic overflow
pub fn with_exo_cr(
  pair: &mut ExoPairState<Clock>,
  target_cr: UFix64<N9>,
) -> Result<()> {
  let supply = pair.context.virtual_stablecoin_supply()?;
  let lower = pair.context.collateral_usd_price.lower;
  pair.context.total_collateral = collateral_for_cr(supply, lower, target_cr)?;
  Ok(())
}
