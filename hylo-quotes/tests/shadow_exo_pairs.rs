//! Exo pair loading against the shadow deployment.
//!
//! Mainnet registers cbBTC alone, so the non-cbBTC path is only exercisable
//! here: the shadow exchange has HYPE registered while the rest of the
//! roster is not, which is exactly the partial snapshot
//! [`ProtocolAccounts::from_fetched`] has to tolerate.
//!
//! Requires `RPC_URL` and `--features shadow`.

#![cfg(feature = "shadow")]

use anchor_lang::prelude::Clock;
use anyhow::{anyhow, Context, Result};
use hylo_idl::tokens::HYPE;
use hylo_quotes::prelude::ProtocolAccounts;
use hylo_quotes::protocol_state::build_exo_pair_state;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

#[tokio::test]
#[ignore = "requires RPC_URL against the shadow deployment"]
async fn registered_pair_loads_beside_unregistered_ones() -> Result<()> {
  let rpc = RpcClient::new(std::env::var("RPC_URL")?);
  let fetched = rpc
    .get_multiple_accounts(&ProtocolAccounts::PUBKEYS)
    .await
    .context("protocol account fetch")?;

  // Unregistered roster pairs come back empty and must not fail the build.
  let accounts = ProtocolAccounts::from_fetched(&fetched)?;
  let hype = accounts
    .hype_pair_accounts
    .as_ref()
    .context("HYPE pair is registered on the shadow deployment")?;

  // The pair state loads from the window the bulk fetch collected. Whole
  // `ProtocolState` construction is not asserted here: it also needs an LST
  // context, whose total SOL cache lags on the shadow deployment.
  let clock: Clock = bincode::deserialize(&accounts.clock.data)
    .map_err(|e| anyhow!("Failed to deserialize clock: {e}"))?;
  let pair = build_exo_pair_state::<HYPE>(
    clock,
    &hype.exo_pair,
    &hype.vault,
    &hype.levercoin_mint,
    &hype.collateral_usd_pyth,
  )?;
  assert!(pair.oracle_interval_secs > 0);
  Ok(())
}
