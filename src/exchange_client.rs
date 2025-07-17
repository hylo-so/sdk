use crate::exchange::client::{accounts, args};
use crate::exchange::events::ExchangeStats;
use crate::exchange::types::SlippageConfig;
use crate::util::{
  simulation_config, ProgramClient, EXCHANGE_LOOKUP_TABLE,
  LST_REGISTRY_LOOKUP_TABLE, SOL_USD_PYTH_FEED,
};
use crate::{exchange, pda, stability_pool};

use std::sync::Arc;

use anchor_client::solana_sdk::address_lookup_table::program::ID as LOOKUP_TABLE_PROGRAM;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::Program;
use anchor_lang::{system_program, AnchorDeserialize};
use anchor_spl::{associated_token, token};
use anyhow::{anyhow, Result};
use base64::prelude::{Engine, BASE64_STANDARD};
use fix::prelude::*;

pub struct ExchangeClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for ExchangeClient {
  const PROGRAM_ID: Pubkey = exchange::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> ExchangeClient {
    ExchangeClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl ExchangeClient {
  /// Mints stablecoin against the given LST.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn mint_stablecoin(
    &self,
    amount_lst: UFix64<N9>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<Signature> {
    let accounts = accounts::MintStablecoin {
      user,
      hylo: pda::hylo(),
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      stablecoin_auth: pda::hyusd_auth(),
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_header: pda::lst_header(lst_mint),
      user_lst_ata: pda::ata(user, lst_mint),
      user_stablecoin_ata: pda::ata(user, pda::hyusd()),
      lst_mint,
      stablecoin_mint: pda::hyusd(),
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      system_program: system_program::ID,
      event_authority: pda::event_auth(exchange::ID),
      program: exchange::ID,
    };
    let args = args::MintStablecoin {
      amount_lst_to_deposit: amount_lst.bits,
      slippage_config,
    };
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables =
      self.load_lookup_tables(&[EXCHANGE_LOOKUP_TABLE]).await?;
    let sig = self
      .send_v0_transaction(&instructions, &lookup_tables)
      .await?;
    Ok(sig)
  }

  /// Redeems stablecoin into the given LST.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn redeem_stablecoin(
    &self,
    amount_stablecoin: UFix64<N6>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<Signature> {
    let accounts = accounts::RedeemStablecoin {
      user,
      hylo: pda::hylo(),
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      stablecoin_auth: pda::hyusd_auth(),
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_header: pda::lst_header(lst_mint),
      user_stablecoin_ata: pda::ata(user, pda::hyusd()),
      user_lst_ata: pda::ata(user, lst_mint),
      stablecoin_mint: pda::hyusd(),
      lst_mint,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: pda::event_auth(exchange::ID),
      program: exchange::ID,
    };
    let args = args::RedeemStablecoin {
      amount_to_redeem: amount_stablecoin.bits,
      slippage_config,
    };
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables =
      self.load_lookup_tables(&[EXCHANGE_LOOKUP_TABLE]).await?;
    let sig = self
      .send_v0_transaction(&instructions, &lookup_tables)
      .await?;
    Ok(sig)
  }

  /// Mints levercoin against the given LST.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn mint_levercoin(
    &self,
    amount_lst: UFix64<N9>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<Signature> {
    let accounts = accounts::MintLevercoin {
      user,
      hylo: pda::hylo(),
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      levercoin_auth: pda::xsol_auth(),
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_header: pda::lst_header(lst_mint),
      user_lst_ata: pda::ata(user, lst_mint),
      user_levercoin_ata: pda::ata(user, pda::xsol()),
      lst_mint,
      levercoin_mint: pda::xsol(),
      stablecoin_mint: pda::hyusd(),
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      system_program: system_program::ID,
      event_authority: pda::event_auth(exchange::ID),
      program: exchange::ID,
    };
    let args = args::MintLevercoin {
      amount_lst_to_deposit: amount_lst.bits,
      slippage_config,
    };
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables =
      self.load_lookup_tables(&[EXCHANGE_LOOKUP_TABLE]).await?;
    let sig = self
      .send_v0_transaction(&instructions, &lookup_tables)
      .await?;
    Ok(sig)
  }

  /// Redeems levercoin into the given LST.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn redeem_levercoin(
    &self,
    amount_levercoin: UFix64<N6>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<Signature> {
    let accounts = accounts::RedeemLevercoin {
      user,
      hylo: pda::hylo(),
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      levercoin_auth: pda::xsol_auth(),
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_header: pda::lst_header(lst_mint),
      user_levercoin_ata: pda::ata(user, pda::xsol()),
      user_lst_ata: pda::ata(user, lst_mint),
      levercoin_mint: pda::xsol(),
      stablecoin_mint: pda::hyusd(),
      lst_mint,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: pda::event_auth(exchange::ID),
      program: exchange::ID,
    };
    let args = args::RedeemLevercoin {
      amount_to_redeem: amount_levercoin.bits,
      slippage_config,
    };
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables =
      self.load_lookup_tables(&[EXCHANGE_LOOKUP_TABLE]).await?;
    let sig = self
      .send_v0_transaction(&instructions, &lookup_tables)
      .await?;
    Ok(sig)
  }

  /// Runs exchange's LST price oracle crank.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn update_lst_prices(&self) -> Result<Signature> {
    let accounts = accounts::UpdateLstPrices {
      payer: self.program.payer(),
      hylo: pda::hylo(),
      lst_registry: LST_REGISTRY_LOOKUP_TABLE,
      lut_program: LOOKUP_TABLE_PROGRAM,
      event_authority: pda::event_auth(exchange::ID),
      program: exchange::ID,
    };
    let args = args::UpdateLstPrices {};
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_lookup_tables(&[EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE])
      .await?;
    let sig = self
      .send_v0_transaction(&instructions, &lookup_tables)
      .await?;
    Ok(sig)
  }

  /// Harvests yield from LST vaults to stability pool.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn harvest_yield(&self) -> Result<Signature> {
    let accounts = accounts::HarvestYield {
      payer: self.program.payer(),
      hylo: pda::hylo(),
      stablecoin_mint: pda::hyusd(),
      stablecoin_auth: pda::hyusd_auth(),
      levercoin_mint: pda::xsol(),
      levercoin_auth: pda::xsol_auth(),
      fee_auth: pda::fee_auth(pda::hyusd()),
      fee_vault: pda::fee_vault(pda::hyusd()),
      stablecoin_pool: pda::hyusd_pool(),
      levercoin_pool: pda::xsol_pool(),
      pool_auth: pda::pool_auth(),
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_stability_pool: stability_pool::ID,
      lst_registry: LST_REGISTRY_LOOKUP_TABLE,
      lut_program: LOOKUP_TABLE_PROGRAM,
      associated_token_program: associated_token::ID,
      token_program: token::ID,
      system_program: system_program::ID,
      event_authority: pda::event_auth(exchange::ID),
      program: exchange::ID,
    };
    let args = args::HarvestYield {};
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_lookup_tables(&[EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE])
      .await?;
    let sig = self
      .send_v0_transaction(&instructions, &lookup_tables)
      .await?;
    Ok(sig)
  }

  /// Simulates the `get_stats` instruction on the exchange.
  ///
  /// # Errors
  /// - Simulation failure
  /// - Return data access or deserialization
  pub async fn simulate_get_stats(&self) -> Result<ExchangeStats> {
    let accounts = accounts::GetStats {
      hylo: pda::hylo(),
      stablecoin_mint: pda::hyusd(),
      levercoin_mint: pda::xsol(),
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
    let result = rpc
      .simulate_transaction_with_config(&tx, simulation_config())
      .await?;
    let (data, _) = result
      .value
      .return_data
      .ok_or(anyhow!("No return data for `get_stats`"))?
      .data;
    let bytes = BASE64_STANDARD.decode(data)?;
    let stats = ExchangeStats::try_from_slice(&bytes)?;
    Ok(stats)
  }
}
