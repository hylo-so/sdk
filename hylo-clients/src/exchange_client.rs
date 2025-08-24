use crate::program_client::{ProgramClient, VersionedTransactionArgs};
use crate::util::{EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE};
use hylo_core::pyth::SOL_USD_PYTH_FEED;
use hylo_idl::exchange::client::{accounts, args};
use hylo_idl::exchange::events::ExchangeStats;
use hylo_idl::exchange::types::SlippageConfig;
use hylo_idl::{ata, exchange, pda, stability_pool};

use std::sync::Arc;

use anchor_client::solana_sdk::address_lookup_table::program::ID as LOOKUP_TABLE_PROGRAM;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::Program;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};
use anyhow::Result;
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
  /// Builds `mint_hyusd` transaction arguments.
  ///
  /// # Errors
  /// - Failed to build instructions or load lookup tables
  pub async fn mint_hyusd_args(
    &self,
    amount_lst: UFix64<N9>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<VersionedTransactionArgs> {
    let accounts = accounts::MintStablecoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_header: pda::lst_header(lst_mint),
      user_lst_ata: ata!(user, lst_mint),
      user_stablecoin_ata: pda::hyusd_ata(user),
      lst_mint,
      stablecoin_mint: *pda::HYUSD,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
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
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionArgs {
      instructions,
      lookup_tables,
    })
  }

  /// Mints hyUSD from the given LST.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn mint_hyusd(
    &self,
    amount_lst: UFix64<N9>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<Signature> {
    let args = self
      .mint_hyusd_args(amount_lst, lst_mint, user, slippage_config)
      .await?;
    let tx = self.build_v0_transaction(&args).await?;
    let sig = self
      .program()
      .rpc()
      .send_and_confirm_transaction(&tx)
      .await?;
    Ok(sig)
  }

  /// Builds `redeem_hyusd` transaction arguments.
  ///
  /// # Errors
  /// - Failed to build instructions or load lookup tables
  pub async fn redeem_hyusd_args(
    &self,
    amount_hyusd: UFix64<N6>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<VersionedTransactionArgs> {
    let accounts = accounts::RedeemStablecoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_header: pda::lst_header(lst_mint),
      user_stablecoin_ata: pda::hyusd_ata(user),
      user_lst_ata: ata!(user, lst_mint),
      stablecoin_mint: *pda::HYUSD,
      lst_mint,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };
    let args = args::RedeemStablecoin {
      amount_to_redeem: amount_hyusd.bits,
      slippage_config,
    };
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionArgs {
      instructions,
      lookup_tables,
    })
  }

  /// Redeems hyUSD into the given LST.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn redeem_hyusd(
    &self,
    amount_hyusd: UFix64<N6>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<Signature> {
    let args = self
      .redeem_hyusd_args(amount_hyusd, lst_mint, user, slippage_config)
      .await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Builds `mint_xsol` transaction arguments.
  ///
  /// # Errors
  /// - Failed to build instructions or load lookup tables
  pub async fn mint_xsol_args(
    &self,
    amount_lst: UFix64<N9>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<VersionedTransactionArgs> {
    let accounts = accounts::MintLevercoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      levercoin_auth: *pda::XSOL_AUTH,
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_header: pda::lst_header(lst_mint),
      user_lst_ata: ata!(user, lst_mint),
      user_levercoin_ata: pda::xsol_ata(user),
      lst_mint,
      levercoin_mint: *pda::XSOL,
      stablecoin_mint: *pda::HYUSD,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
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
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionArgs {
      instructions,
      lookup_tables,
    })
  }

  /// Mints xSOL against the given LST.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn mint_xsol(
    &self,
    amount_lst: UFix64<N9>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<Signature> {
    let args = self
      .mint_xsol_args(amount_lst, lst_mint, user, slippage_config)
      .await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Builds `redeem_xsol` transaction arguments.
  ///
  /// # Errors
  /// - Failed to build instructions or load lookup tables
  pub async fn redeem_xsol_args(
    &self,
    amount_xsol: UFix64<N6>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<VersionedTransactionArgs> {
    let accounts = accounts::RedeemLevercoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(lst_mint),
      vault_auth: pda::vault_auth(lst_mint),
      levercoin_auth: *pda::XSOL_AUTH,
      fee_vault: pda::fee_vault(lst_mint),
      lst_vault: pda::vault(lst_mint),
      lst_header: pda::lst_header(lst_mint),
      user_levercoin_ata: pda::xsol_ata(user),
      user_lst_ata: ata!(user, lst_mint),
      levercoin_mint: *pda::XSOL,
      stablecoin_mint: *pda::HYUSD,
      lst_mint,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };
    let args = args::RedeemLevercoin {
      amount_to_redeem: amount_xsol.bits,
      slippage_config,
    };
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionArgs {
      instructions,
      lookup_tables,
    })
  }

  /// Redeems xSOL into the given LST.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn redeem_xsol(
    &self,
    amount_xsol: UFix64<N6>,
    lst_mint: Pubkey,
    user: Pubkey,
    slippage_config: Option<SlippageConfig>,
  ) -> Result<Signature> {
    let args = self
      .redeem_xsol_args(amount_xsol, lst_mint, user, slippage_config)
      .await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Builds `swap_hyusd_to_xsol` transaction arguments.
  ///
  /// # Errors
  /// - Failed to build instructions or load lookup tables
  pub async fn swap_hyusd_to_xsol_args(
    &self,
    amount_hyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<VersionedTransactionArgs> {
    let accounts = accounts::SwapStableToLever {
      user,
      hylo: *pda::HYLO,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_mint: *pda::HYUSD,
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_auth: pda::fee_auth(*pda::HYUSD),
      fee_vault: pda::fee_vault(*pda::HYUSD),
      user_stablecoin_ata: pda::hyusd_ata(user),
      levercoin_mint: *pda::XSOL,
      levercoin_auth: *pda::XSOL_AUTH,
      user_levercoin_ata: pda::xsol_ata(user),
      token_program: token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };
    let args = args::SwapStableToLever {
      amount_stablecoin: amount_hyusd.bits,
    };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables =
      vec![self.load_lookup_table(&EXCHANGE_LOOKUP_TABLE).await?];
    Ok(VersionedTransactionArgs {
      instructions,
      lookup_tables,
    })
  }

  /// Swaps hyUSD to xSOL.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn swap_hyusd_to_xsol(
    &self,
    amount_hyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<Signature> {
    let args = self.swap_hyusd_to_xsol_args(amount_hyusd, user).await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Builds `swap_xsol_to_hyusd` transaction arguments.
  ///
  /// # Errors
  /// - Failed to build instructions or load lookup tables
  pub async fn swap_xsol_to_hyusd_args(
    &self,
    amount_xsol: UFix64<N6>,
    user: Pubkey,
  ) -> Result<VersionedTransactionArgs> {
    let accounts = accounts::SwapLeverToStable {
      user,
      hylo: *pda::HYLO,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_mint: *pda::HYUSD,
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_auth: pda::fee_auth(*pda::HYUSD),
      fee_vault: pda::fee_vault(*pda::HYUSD),
      user_stablecoin_ata: pda::hyusd_ata(user),
      levercoin_mint: *pda::XSOL,
      levercoin_auth: *pda::XSOL_AUTH,
      user_levercoin_ata: pda::xsol_ata(user),
      token_program: token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };
    let args = args::SwapLeverToStable {
      amount_levercoin: amount_xsol.bits,
    };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables =
      vec![self.load_lookup_table(&EXCHANGE_LOOKUP_TABLE).await?];
    Ok(VersionedTransactionArgs {
      instructions,
      lookup_tables,
    })
  }

  /// Swaps xSOL to hyUSD.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn swap_xsol_to_hyusd(
    &self,
    amount_xsol: UFix64<N6>,
    user: Pubkey,
  ) -> Result<Signature> {
    let args = self
      .swap_xsol_to_hyusd_args(amount_xsol, user)
      .await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Runs exchange's LST price oracle crank.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn update_lst_prices(&self) -> Result<Signature> {
    let accounts = accounts::UpdateLstPrices {
      payer: self.program.payer(),
      hylo: *pda::HYLO,
      lst_registry: LST_REGISTRY_LOOKUP_TABLE,
      lut_program: LOOKUP_TABLE_PROGRAM,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };
    let args = args::UpdateLstPrices {};
    let (remaining_accounts, registry_lut) = self.load_lst_registry().await?;
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .accounts(remaining_accounts)
      .args(args)
      .instructions()?;
    let exchange_lut = self.load_lookup_table(&EXCHANGE_LOOKUP_TABLE).await?;
    let lookup_tables = vec![registry_lut, exchange_lut];
    let args = VersionedTransactionArgs {
      instructions,
      lookup_tables,
    };
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Harvests yield from LST vaults to stability pool.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn harvest_yield(&self) -> Result<Signature> {
    let accounts = accounts::HarvestYield {
      payer: self.program.payer(),
      hylo: *pda::HYLO,
      stablecoin_mint: *pda::HYUSD,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_mint: *pda::XSOL,
      levercoin_auth: *pda::XSOL_AUTH,
      fee_auth: pda::fee_auth(*pda::HYUSD),
      fee_vault: pda::fee_vault(*pda::HYUSD),
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      pool_auth: *pda::POOL_AUTH,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_stability_pool: stability_pool::ID,
      lst_registry: LST_REGISTRY_LOOKUP_TABLE,
      lut_program: LOOKUP_TABLE_PROGRAM,
      associated_token_program: associated_token::ID,
      token_program: token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };
    let args = args::HarvestYield {};
    let (remaining_accounts, registry_lut) = self.load_lst_registry().await?;
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .accounts(remaining_accounts)
      .args(args)
      .instructions()?;
    let exchange_lut = self.load_lookup_table(&EXCHANGE_LOOKUP_TABLE).await?;
    let lookup_tables = vec![registry_lut, exchange_lut];
    let args = VersionedTransactionArgs {
      instructions,
      lookup_tables,
    };
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  /// Gets exchange stats via RPC simulation.
  ///
  /// # Errors
  /// - Failed to simulate transaction
  /// - Failed to deserialize return data
  pub async fn get_stats(&self) -> Result<ExchangeStats> {
    let accounts = accounts::GetStats {
      hylo: *pda::HYLO,
      stablecoin_mint: *pda::HYUSD,
      levercoin_mint: *pda::XSOL,
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
