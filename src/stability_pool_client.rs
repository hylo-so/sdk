use crate::pda::SOL_USD_PYTH_FEED;
use crate::stability_pool::client::{accounts, args};
use crate::stability_pool::events::StabilityPoolStats;
use crate::util::{simulation_config, ProgramClient};
use crate::{exchange, pda, stability_pool};

use std::rc::Rc;

use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anchor_lang::AnchorDeserialize;
use anchor_spl::token;
use anyhow::{anyhow, Result};
use base64::prelude::{Engine, BASE64_STANDARD};

pub struct StabilityPoolClient {
  program: Program<Rc<Keypair>>,
}

impl ProgramClient for StabilityPoolClient {
  const PROGRAM_ID: Pubkey = stability_pool::ID;

  fn build_client(program: Program<Rc<Keypair>>) -> StabilityPoolClient {
    StabilityPoolClient { program }
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
      pool_config: pda::pool_config(),
      hylo: pda::hylo(),
      stablecoin_mint: pda::hyusd(),
      stablecoin_pool: pda::hyusd_pool(),
      pool_auth: pda::pool_auth(),
      levercoin_pool: pda::xsol_pool(),
      fee_auth: pda::fee_auth(pda::hyusd()),
      fee_vault: pda::fee_vault(pda::hyusd()),
      levercoin_mint: pda::xsol(),
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_auth: pda::hyusd_auth(),
      levercoin_auth: pda::xsol_auth(),
      hylo_event_authority: pda::event_auth(exchange::ID),
      hylo_exchange_program: exchange::ID,
      token_program: token::ID,
      event_authority: pda::event_auth(stability_pool::ID),
      program: stability_pool::ID,
    };
    let args = args::RebalanceStableToLever {};
    let sig = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .send()
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
      pool_config: pda::pool_config(),
      hylo: pda::hylo(),
      stablecoin_mint: pda::hyusd(),
      stablecoin_pool: pda::hyusd_pool(),
      pool_auth: pda::pool_auth(),
      levercoin_pool: pda::xsol_pool(),
      fee_auth: pda::fee_auth(pda::hyusd()),
      fee_vault: pda::fee_vault(pda::hyusd()),
      levercoin_mint: pda::xsol(),
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_auth: pda::hyusd_auth(),
      levercoin_auth: pda::xsol_auth(),
      hylo_event_authority: pda::event_auth(exchange::ID),
      hylo_exchange_program: exchange::ID,
      token_program: token::ID,
      event_authority: pda::event_auth(stability_pool::ID),
      program: stability_pool::ID,
    };
    let args = args::RebalanceLeverToStable {};
    let sig = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .send()
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
      pool_config: pda::pool_config(),
      hylo: pda::hylo(),
      stablecoin_mint: pda::hyusd(),
      levercoin_mint: pda::xsol(),
      pool_auth: pda::pool_auth(),
      stablecoin_pool: pda::hyusd_pool(),
      levercoin_pool: pda::xsol_pool(),
      lp_token_mint: pda::shyusd(),
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
