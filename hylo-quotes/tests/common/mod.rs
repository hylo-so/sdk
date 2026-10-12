//! Shared fixtures for snapshot-based tests.

use std::fs::File;

use anchor_lang::solana_program::clock::Clock;
use anyhow::Result;
use hylo_quotes::prelude::{ProtocolAccounts, ProtocolState};
use serde_json::from_reader;

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
