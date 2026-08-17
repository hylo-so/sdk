//! Smoke tests quoting every route against a mainnet account snapshot.
//!
//! Prices move with every snapshot, so these assert that a route produces
//! output, not what it produces.

use std::fs::File;

use anchor_lang::solana_program::clock::Clock;
use anyhow::Result;
use fix::prelude::*;
use hylo_clients::prelude::CommitmentConfig;
use hylo_idl::tokens::{HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_quotes::prelude::{
  ProtocolAccounts, ProtocolState, TokenOperationExt,
};
use serde_json::{from_reader, to_writer};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

/// Pulls needed accounts from RPC into a file indexed by epoch and slot.
///
/// # Errors
/// * RPC call
/// * Protocol accounts construction
/// * File IO
pub async fn dump_protocol_accounts() -> Result<()> {
  let rpc_client = RpcClient::new_with_commitment(
    "https://api.mainnet-beta.solana.com".to_string(),
    CommitmentConfig::confirmed(),
  );
  let accounts = rpc_client
    .get_multiple_accounts(&ProtocolAccounts::PUBKEYS)
    .await?;
  let epoch = rpc_client.get_epoch_info().await?;
  let filename = format!(
    "tests/data/protocol-state-{}-{}.json",
    epoch.epoch, epoch.slot_index
  );
  let protocol_accounts = ProtocolAccounts::from_fetched(&accounts)?;
  let file = File::create_new(filename)?;
  to_writer(file, &protocol_accounts)?;
  Ok(())
}

#[tokio::test]
#[ignore = "writes a new snapshot into tests/data"]
async fn dump_snapshot() -> Result<()> {
  dump_protocol_accounts().await
}

fn load_state() -> Result<ProtocolState<Clock>> {
  let path = format!(
    "{}/tests/data/protocol-state-1018-114971.json",
    env!("CARGO_MANIFEST_DIR")
  );
  let file = File::open(path)?;
  let accounts = from_reader::<_, ProtocolAccounts>(file)?;
  ProtocolState::try_from(&accounts)
}

#[test]
fn jitosol_to_xsol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N9>::new(1_000_000_000);
  let op = state.output::<JITOSOL, XSOL>(amount_in)?;
  assert!(op.out_amount > UFix64::<N6>::new(0));
  Ok(())
}

#[test]
fn xsol_to_jitosol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N6>::new(1_000_000);
  let op = state.output::<XSOL, JITOSOL>(amount_in)?;
  assert!(op.out_amount > UFix64::<N9>::new(0));
  Ok(())
}

#[test]
fn hyusd_to_xsol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N6>::new(1_000_000);
  let op = state.output::<HYUSD, XSOL>(amount_in)?;
  assert!(op.out_amount > UFix64::<N6>::new(0));
  Ok(())
}

#[test]
fn jitosol_to_hylosol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N9>::new(1_000_000_000);
  let op = state.output::<JITOSOL, HYLOSOL>(amount_in)?;
  assert!(op.out_amount > UFix64::<N9>::new(0));
  Ok(())
}

#[test]
fn hyusd_to_shyusd() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N6>::new(1_000_000);
  let op = state.output::<HYUSD, SHYUSD>(amount_in)?;
  assert!(op.out_amount > UFix64::<N6>::new(0));
  Ok(())
}
