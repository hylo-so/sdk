use crate::program_client::ProgramClient;
use crate::util::{
  simulation_config, EXCHANGE_LOOKUP_TABLE, STABILITY_POOL_LOOKUP_TABLE,
};
use hylo_core::pyth::SOL_USD_PYTH_FEED;

use std::sync::Arc;

use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::Program;
use anchor_lang::prelude::{AnchorDeserialize, Pubkey};
use anchor_spl::token;
use anyhow::{anyhow, Result};
use base64::prelude::{Engine, BASE64_STANDARD};
use hylo_idl::stability_pool::client::{accounts, args};
use hylo_idl::stability_pool::events::StabilityPoolStats;
use hylo_idl::{exchange, pda, stability_pool};

pub struct StabilityPoolClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for StabilityPoolClient {
  const PROGRAM_ID: Pubkey = stability_pool::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> StabilityPoolClient {
    StabilityPoolClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl StabilityPoolClient {
  /// Rebalances stability pool by swapping stablecoin to levercoin.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn rebalance_stable_to_lever(&self) -> Result<Signature> {
    let accounts = accounts::RebalanceStableToLever {
      payer: self.program.payer(),
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: *pda::HYUSD,
      stablecoin_pool: *pda::HYUSD_POOL,
      pool_auth: *pda::POOL_AUTH,
      levercoin_pool: *pda::XSOL_POOL,
      fee_auth: pda::fee_auth(*pda::HYUSD),
      fee_vault: pda::fee_vault(*pda::HYUSD),
      levercoin_mint: *pda::XSOL,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      token_program: token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::RebalanceStableToLever {};
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    let sig = self
      .send_v0_transaction(&instructions, &lookup_tables)
      .await?;
    Ok(sig)
  }

  /// Rebalances levercoin from the stability pool back to stablecoin.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn rebalance_lever_to_stable(&self) -> Result<Signature> {
    let accounts = accounts::RebalanceLeverToStable {
      payer: self.program.payer(),
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: *pda::HYUSD,
      stablecoin_pool: *pda::HYUSD_POOL,
      pool_auth: *pda::POOL_AUTH,
      levercoin_pool: *pda::XSOL_POOL,
      fee_auth: pda::fee_auth(*pda::HYUSD),
      fee_vault: pda::fee_vault(*pda::HYUSD),
      levercoin_mint: *pda::XSOL,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      token_program: token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::RebalanceLeverToStable {};
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    let sig = self
      .send_v0_transaction(&instructions, &lookup_tables)
      .await?;
    Ok(sig)
  }

  /// Simulates the `get_stats` instruction on the stability pool.
  ///
  /// # Errors
  /// - Simulation failure
  /// - Return data access or deserialization
  pub async fn simulate_get_stats(&self) -> Result<StabilityPoolStats> {
    let accounts = accounts::GetStats {
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: *pda::HYUSD,
      levercoin_mint: *pda::XSOL,
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      lp_token_mint: *pda::SHYUSD,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    };
    let args = args::GetStats {};
    let tx = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .signed_transaction()
      .await?;
    let rpc = self.program.rpc();
    let (data, _) = rpc
      .simulate_transaction_with_config(&tx, simulation_config())
      .await?
      .value
      .return_data
      .ok_or(anyhow!("No return data for `get_stats`"))?
      .data;
    let bytes = BASE64_STANDARD.decode(data)?;
    let stats = StabilityPoolStats::try_from_slice(&bytes)?;
    Ok(stats)
  }
}
