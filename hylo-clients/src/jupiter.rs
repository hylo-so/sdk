use anchor_lang::{prelude::Pubkey, AnchorDeserialize};
use anchor_spl::token::Mint;
use anyhow::{anyhow, Result};
use base64::prelude::{Engine, BASE64_STANDARD};
use fix::prelude::*;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fee_controller::{LevercoinFees, StablecoinFees};
use hylo_core::pyth::OracleConfig;
use hylo_core::stability_mode::StabilityController;
use hylo_core::total_sol_cache::TotalSolCache;
use jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
  SwapAndAccountMetas, SwapParams,
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::exchange::accounts::{Hylo, LstHeader};
use crate::hylo_exchange;
use crate::pda;
use crate::util::{get_account, JITOSOL_MINT, SOL_USD_PYTH_FEED};

#[derive(Clone)]
pub struct HyloExchangeState {
  clock: ClockRef,
  total_sol_cache: TotalSolCache,
  stability_controller: StabilityController,
  oracle_config: OracleConfig<N8>,
  stablecoin_fees: StablecoinFees,
  levercoin_fees: LevercoinFees,
  stablecoin_mint: Option<Mint>,
  levercoin_mint: Option<Mint>,
  jitosol_header: Option<LstHeader>,
  sol_usd: Option<PriceUpdateV2>,
}

impl HyloExchangeState {
  fn sol_usd(&self) -> Result<&PriceUpdateV2> {
    self.sol_usd.as_ref().ok_or(anyhow!("sol_usd not set"))
  }

  fn stablecoin_mint(&self) -> Result<&Mint> {
    self
      .stablecoin_mint
      .as_ref()
      .ok_or(anyhow!("stablecoin_mint not set"))
  }

  fn levercoin_mint(&self) -> Result<&Mint> {
    self
      .levercoin_mint
      .as_ref()
      .ok_or(anyhow!("levercoin_mint not set"))
  }

  fn jitosol_header(&self) -> Result<&LstHeader> {
    self
      .jitosol_header
      .as_ref()
      .ok_or(anyhow!("jitosol_header not set"))
  }
}

impl Amm for HyloExchangeState {
  fn from_keyed_account(
    keyed_account: &KeyedAccount,
    amm_context: &AmmContext,
  ) -> Result<Self>
  where
    Self: Sized,
  {
    let bytes = BASE64_STANDARD.decode(keyed_account.account.data.clone())?;
    let hylo = Hylo::try_from_slice(&bytes)?;
    let oracle_config = OracleConfig::new(
      hylo.oracle_interval_secs,
      Into::<UFixValue64>::into(hylo.oracle_conf_tolerance).try_into()?,
    );
    let stability_controller = StabilityController::new(
      Into::<UFixValue64>::into(hylo.stability_threshold_1).try_into()?,
      Into::<UFixValue64>::into(hylo.stability_threshold_2).try_into()?,
    )?;
    Ok(HyloExchangeState {
      clock: amm_context.clock_ref.clone(),
      total_sol_cache: hylo.total_sol_cache.into(),
      stability_controller,
      oracle_config,
      stablecoin_fees: hylo.stablecoin_fees.into(),
      levercoin_fees: hylo.levercoin_fees.into(),
      stablecoin_mint: None,
      levercoin_mint: None,
      jitosol_header: None,
      sol_usd: None,
    })
  }

  fn label(&self) -> String {
    "Hylo Exchange".to_string()
  }

  fn program_id(&self) -> Pubkey {
    hylo_exchange::ID
  }

  fn key(&self) -> Pubkey {
    *pda::HYLO
  }

  fn get_reserve_mints(&self) -> Vec<Pubkey> {
    vec![*pda::HYUSD, *pda::XSOL, JITOSOL_MINT]
  }

  fn get_accounts_to_update(&self) -> Vec<Pubkey> {
    vec![
      *pda::HYUSD,
      *pda::XSOL,
      pda::lst_header(JITOSOL_MINT),
      SOL_USD_PYTH_FEED,
    ]
  }

  fn update(&mut self, account_map: &AccountMap) -> Result<()> {
    let stablecoin_mint: Mint = get_account(account_map, &pda::HYUSD)?;
    let levercoin_mint: Mint = get_account(account_map, &pda::XSOL)?;
    let jitosol_header: LstHeader =
      get_account(account_map, &pda::lst_header(JITOSOL_MINT))?;
    let sol_usd: PriceUpdateV2 = get_account(account_map, &SOL_USD_PYTH_FEED)?;
    self.stablecoin_mint = Some(stablecoin_mint);
    self.levercoin_mint = Some(levercoin_mint);
    self.jitosol_header = Some(jitosol_header);
    self.sol_usd = Some(sol_usd);
    Ok(())
  }

  fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
    let exchange_context = ExchangeContext::load(
      self.clock.clone(),
      &self.total_sol_cache,
      self.stability_controller.clone(),
      self.oracle_config.clone(),
      self.stablecoin_fees,
      self.levercoin_fees,
      self.sol_usd()?,
      self.stablecoin_mint()?,
      self.levercoin_mint()?,
    )?;
    todo!()
  }

  fn get_swap_and_account_metas(
    &self,
    swap_params: &SwapParams,
  ) -> Result<SwapAndAccountMetas> {
    todo!()
  }

  fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
    Box::new(self.clone())
  }
}

#[cfg(test)]
mod tests {}
