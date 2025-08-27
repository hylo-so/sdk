use crate::program_client::{ProgramClient, VersionedTransactionArgs};
use crate::util::{EXCHANGE_LOOKUP_TABLE, STABILITY_POOL_LOOKUP_TABLE};
use hylo_core::pyth::SOL_USD_PYTH_FEED;

use std::sync::Arc;

use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};
use anyhow::Result;
use fix::prelude::{UFix64, N6};
use hylo_idl::pda::{HYUSD, SHYUSD, XSOL};
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
  /// Creates transaction arguments for sHYUSD mint aka `user_deposit`.
  ///
  /// # Errors
  /// - Transaction building failure
  pub async fn mint_shyusd_args(
    &self,
    amount_hyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<VersionedTransactionArgs> {
    let accounts = accounts::UserDeposit {
      user,
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD,
      levercoin_mint: XSOL,
      user_stablecoin_ata: pda::hyusd_ata(user),
      user_lp_token_ata: pda::shyusd_ata(user),
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      lp_token_auth: *pda::SHYUSD_AUTH,
      lp_token_mint: SHYUSD,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::UserDeposit {
      amount_stablecoin: amount_hyusd.bits,
    };
    let instructions = self
      .program()
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
    Ok(VersionedTransactionArgs {
      instructions,
      lookup_tables,
    })
  }

  /// Mints sHYUSD LP tokens by depositing hyUSD.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn mint_shyusd(
    &self,
    amount_hyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<Signature> {
    let args = self.mint_shyusd_args(amount_hyusd, user).await?;
    let tx = self.build_v0_transaction(&args).await?;
    let sig = self
      .program()
      .rpc()
      .send_and_confirm_transaction(&tx)
      .await?;
    Ok(sig)
  }

  /// Creates transaction arguments for redeeming sHYUSD LP tokens.
  ///
  /// # Errors
  /// - Transaction building failure
  pub async fn redeem_shyusd_args(
    &self,
    amount_shyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<VersionedTransactionArgs> {
    let accounts = accounts::UserWithdraw {
      user,
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD,
      user_stablecoin_ata: pda::hyusd_ata(user),
      fee_auth: pda::fee_auth(HYUSD),
      fee_vault: pda::fee_vault(HYUSD),
      user_lp_token_ata: pda::shyusd_ata(user),
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_mint: XSOL,
      levercoin_pool: *pda::XSOL_POOL,
      user_levercoin_ata: pda::xsol_ata(user),
      lp_token_auth: *pda::SHYUSD_AUTH,
      lp_token_mint: SHYUSD,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::UserWithdraw {
      amount_lp_token: amount_shyusd.bits,
    };
    let instructions = self
      .program()
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
    Ok(VersionedTransactionArgs {
      instructions,
      lookup_tables,
    })
  }

  /// Redeems sHYUSD LP tokens for hyUSD.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn redeem_shyusd(
    &self,
    amount_shyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<Signature> {
    let args = self.redeem_shyusd_args(amount_shyusd, user).await?;
    let tx = self.build_v0_transaction(&args).await?;
    let sig = self
      .program()
      .rpc()
      .send_and_confirm_transaction(&tx)
      .await?;
    Ok(sig)
  }

  /// Rebalances stability pool by swapping stablecoin to levercoin.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn rebalance_stable_to_lever(&self) -> Result<Signature> {
    let accounts = accounts::RebalanceStableToLever {
      payer: self.program.payer(),
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD,
      stablecoin_pool: *pda::HYUSD_POOL,
      pool_auth: *pda::POOL_AUTH,
      levercoin_pool: *pda::XSOL_POOL,
      fee_auth: pda::fee_auth(HYUSD),
      fee_vault: pda::fee_vault(HYUSD),
      levercoin_mint: XSOL,
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
    let tx_args = VersionedTransactionArgs {
      instructions,
      lookup_tables,
    };
    let sig = self.send_v0_transaction(&tx_args).await?;
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
      stablecoin_mint: HYUSD,
      stablecoin_pool: *pda::HYUSD_POOL,
      pool_auth: *pda::POOL_AUTH,
      levercoin_pool: *pda::XSOL_POOL,
      fee_auth: pda::fee_auth(HYUSD),
      fee_vault: pda::fee_vault(HYUSD),
      levercoin_mint: XSOL,
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
    let tx_args = VersionedTransactionArgs {
      instructions,
      lookup_tables,
    };
    let sig = self.send_v0_transaction(&tx_args).await?;
    Ok(sig)
  }

  /// Simulates the `get_stats` instruction on the stability pool.
  ///
  /// # Errors
  /// - Simulation failure
  /// - Return data access or deserialization
  pub async fn get_stats(&self) -> Result<StabilityPoolStats> {
    let accounts = accounts::GetStats {
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD,
      levercoin_mint: XSOL,
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      lp_token_mint: SHYUSD,
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
    let stats = self.simulate_transaction_return(tx.into()).await?;
    Ok(stats)
  }
}
