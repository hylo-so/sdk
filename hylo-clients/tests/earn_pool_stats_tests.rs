//! Integration test for earn pool yield stats against mainnet.
//! Skips silently unless `RPC_URL` is set (matches CI secrets setup).

use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::Cluster;
use anyhow::Result;
use hylo_clients::earn_pool_client::EarnPoolClient;
use hylo_clients::program_client::ProgramClient;

#[tokio::test]
async fn earn_pool_stats_mainnet() -> Result<()> {
  match std::env::var("RPC_URL") {
    Err(_) => Ok(()),
    Ok(rpc_url) => {
      let ws_url =
        std::env::var("RPC_WS_URL").unwrap_or_else(|_| "wss://unused".into());
      let client = EarnPoolClient::new_random_keypair(
        Cluster::Custom(rpc_url, ws_url),
        CommitmentConfig::confirmed(),
      )?;
      let stats = client.earn_pool_stats().await?;
      assert!(stats.pool_balance.bits > 0, "empty pool");
      assert!(stats.shyusd_supply.bits > 0, "no sHYUSD supply");
      assert!(stats.nav.bits > 0, "zero NAV");
      assert!(stats.naive_apy.is_finite());
      assert!(stats.projected_apy.is_finite());
      assert!(stats.naive_apy >= 0.0);
      assert!(stats.projected_apy >= 0.0);
      assert!(stats.lst_harvest.epoch <= stats.current_epoch);
      assert!(stats.borrow_harvest.epoch <= stats.current_epoch);
      Ok(())
    }
  }
}
