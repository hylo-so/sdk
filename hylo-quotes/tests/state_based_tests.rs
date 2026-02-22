//! State-based tests for pricing accuracy.
//!
//! TODO: Disabled since onchain snapshots still have oracle conf tolerance at
//! N8.

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
  let pubkeys = ProtocolAccounts::pubkeys();
  let rpc_client = RpcClient::new_with_commitment(
    "https://api.mainnet-beta.solana.com".to_string(),
    CommitmentConfig::confirmed(),
  );
  let accounts = rpc_client.get_multiple_accounts(&pubkeys).await?;
  let epoch = rpc_client.get_epoch_info().await?;
  let filename = format!(
    "tests/data/protocol-state-{}-{}.json",
    epoch.epoch, epoch.slot_index
  );
  let protocol_accounts =
    ProtocolAccounts::try_from((pubkeys.as_slice(), accounts.as_slice()))?;
  let file = File::create_new(filename)?;
  to_writer(file, &protocol_accounts)?;
  Ok(())
}

fn load_state() -> Result<ProtocolState<Clock>> {
  let path = format!(
    "{}/tests/data/protocol-state-918-37508.json",
    env!("CARGO_MANIFEST_DIR")
  );
  let file = File::open(path)?;
  let accounts = from_reader::<_, ProtocolAccounts>(file)?;
  ProtocolState::try_from(&accounts)
}

#[test]
#[ignore = "onchain oracle conf tolerance is N8, SDK now expects N9"]
fn jitosol_to_hyusd() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N9>::new(1_000_000_000);
  let op = state.output::<JITOSOL, HYUSD>(amount_in)?;
  assert_eq!(op.out_amount, UFix64::<N6>::new(154_211_899));
  Ok(())
}

#[test]
#[ignore = "onchain oracle conf tolerance is N8, SDK now expects N9"]
fn hyusd_to_jitosol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N6>::new(1_000_000);
  let op = state.output::<HYUSD, JITOSOL>(amount_in)?;
  assert_eq!(op.out_amount, UFix64::<N9>::new(6_434_815));
  Ok(())
}

#[test]
#[ignore = "onchain oracle conf tolerance is N8, SDK now expects N9"]
fn jitosol_to_xsol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N9>::new(1_000_000_000);
  let op = state.output::<JITOSOL, XSOL>(amount_in)?;
  assert_eq!(op.out_amount, UFix64::<N6>::new(322_028_541));
  Ok(())
}

#[test]
#[ignore = "onchain oracle conf tolerance is N8, SDK now expects N9"]
fn xsol_to_jitosol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N6>::new(1_000_000);
  let op = state.output::<XSOL, JITOSOL>(amount_in)?;
  assert_eq!(op.out_amount, UFix64::<N9>::new(2_945_254));
  Ok(())
}

#[test]
#[ignore = "onchain oracle conf tolerance is N8, SDK now expects N9"]
fn hyusd_to_xsol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N6>::new(1_000_000);
  let op = state.output::<HYUSD, XSOL>(amount_in)?;
  assert_eq!(op.out_amount, UFix64::<N6>::new(2_077_779));
  Ok(())
}

#[test]
#[ignore = "onchain oracle conf tolerance is N8, SDK now expects N9"]
fn xsol_to_hyusd() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N6>::new(1_000_000);
  let op = state.output::<XSOL, HYUSD>(amount_in)?;
  assert_eq!(op.out_amount, UFix64::<N6>::new(457_248));
  Ok(())
}

#[test]
#[ignore = "onchain oracle conf tolerance is N8, SDK now expects N9"]
fn jitosol_to_hylosol() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N9>::new(1_000_000_000);
  let op = state.output::<JITOSOL, HYLOSOL>(amount_in)?;
  assert_eq!(op.out_amount, UFix64::<N9>::new(1_212_807_252));
  Ok(())
}

#[test]
#[ignore = "onchain oracle conf tolerance is N8, SDK now expects N9"]
fn hyusd_to_shyusd() -> Result<()> {
  let state = load_state()?;
  let amount_in = UFix64::<N6>::new(1_000_000);
  let op = state.output::<HYUSD, SHYUSD>(amount_in)?;
  assert_eq!(op.out_amount, UFix64::<N6>::new(860_623));
  Ok(())
}
