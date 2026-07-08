//! Integration test for earn pool yield stats against mainnet.
//! Requires `RPC_URL`; run explicitly with `cargo test -- --ignored`
//! (add `--features shadow` to target the shadow deployment).

use anyhow::Result;
use hylo_stats::earn_pool_stats::fetch_earn_pool_stats;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

#[tokio::test]
#[ignore = "re-enable after mainnet is on v2"]
async fn earn_pool_stats_mainnet() -> Result<()> {
  match std::env::var("RPC_URL") {
    Err(_) => Ok(()),
    Ok(rpc_url) => {
      let rpc = RpcClient::new(rpc_url);
      let stats = fetch_earn_pool_stats(&rpc).await?;
      assert!(stats.pool_balance.bits > 0, "empty pool");
      assert!(stats.shyusd_supply.bits > 0, "no sHYUSD supply");
      assert!(stats.nav.bits > 0, "zero NAV");
      assert!(stats.naive_apy.is_finite());
      assert!(stats.projected_apy.is_finite());
      assert!(stats.naive_apy >= 0.0);
      assert!(stats.projected_apy >= 0.0);
      assert!(stats.epochs_per_year > 100.0 && stats.epochs_per_year < 400.0);
      assert!(stats.lst_harvest.epoch <= stats.current_epoch);
      assert!(stats
        .exo_streams
        .iter()
        .all(|stream| stream.harvest.epoch <= stats.current_epoch));
      Ok(())
    }
  }
}
