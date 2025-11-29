use anchor_lang::prelude::{AnchorDeserialize, Pubkey};
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Result};
use fix::prelude::*;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fee_controller::{LevercoinFees, StablecoinFees};
use hylo_core::idl::hylo_exchange::accounts::{Hylo, LstHeader};
use hylo_core::idl::hylo_stability_pool::accounts::PoolConfig;
use hylo_core::idl::tokens::{HYUSD, JITOSOL, SHYUSD, XSOL};
use hylo_core::idl::{hylo_exchange, pda};
use hylo_core::idl_type_bridge::convert_ufixvalue64;
use hylo_core::pyth::{OracleConfig, SOL_USD_PYTH_FEED};
use hylo_core::stability_mode::StabilityController;
use hylo_core::total_sol_cache::TotalSolCache;
use jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
  SwapAndAccountMetas, SwapParams,
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::quote;
use crate::util::account_map_get;

#[derive(Clone)]
pub struct HyloJupiterClient {
  clock: ClockRef,
  total_sol_cache: TotalSolCache,
  stability_controller: StabilityController,
  oracle_config: OracleConfig<N8>,
  hyusd_fees: StablecoinFees,
  xsol_fees: LevercoinFees,
  hyusd_mint: Option<Mint>,
  xsol_mint: Option<Mint>,
  shyusd_mint: Option<Mint>,
  jitosol_header: Option<LstHeader>,
  sol_usd: Option<PriceUpdateV2>,
  hyusd_pool: Option<TokenAccount>,
  xsol_pool: Option<TokenAccount>,
  pool_config: Option<PoolConfig>,
}

impl HyloJupiterClient {
  fn load_exchange_ctx(&self) -> Result<ExchangeContext<ClockRef>> {
    let ctx = ExchangeContext::load(
      self.clock.clone(),
      &self.total_sol_cache,
      self.stability_controller,
      self.oracle_config,
      self.hyusd_fees,
      self.xsol_fees,
      self.sol_usd()?,
      self.hyusd_mint()?,
      self.xsol_mint().ok(),
    )?;
    Ok(ctx)
  }

  fn sol_usd(&self) -> Result<&PriceUpdateV2> {
    self.sol_usd.as_ref().ok_or(anyhow!("`sol_usd` not set"))
  }

  fn hyusd_mint(&self) -> Result<&Mint> {
    self
      .hyusd_mint
      .as_ref()
      .ok_or(anyhow!("`stablecoin_mint` not set"))
  }

  fn xsol_mint(&self) -> Result<&Mint> {
    self
      .xsol_mint
      .as_ref()
      .ok_or(anyhow!("`levercoin_mint` not set"))
  }

  fn jitosol_header(&self) -> Result<&LstHeader> {
    self
      .jitosol_header
      .as_ref()
      .ok_or(anyhow!("`jitosol_header` not set"))
  }

  fn shyusd_mint(&self) -> Result<&Mint> {
    self
      .shyusd_mint
      .as_ref()
      .ok_or(anyhow!("`shyusd_mint` not set"))
  }

  fn hyusd_pool(&self) -> Result<&TokenAccount> {
    self
      .hyusd_pool
      .as_ref()
      .ok_or(anyhow!("`hyusd_pool` not set"))
  }

  fn pool_config(&self) -> Result<&PoolConfig> {
    self
      .pool_config
      .as_ref()
      .ok_or(anyhow!("`pool_config` not set"))
  }

  fn xsol_pool(&self) -> Result<&TokenAccount> {
    self
      .xsol_pool
      .as_ref()
      .ok_or(anyhow!("`xsol_pool` not set"))
  }
}

impl Amm for HyloJupiterClient {
  fn from_keyed_account(
    keyed_account: &KeyedAccount,
    amm_context: &AmmContext,
  ) -> Result<Self>
  where
    Self: Sized,
  {
    let hylo = Hylo::try_from_slice(&keyed_account.account.data[8..])?;
    let oracle_config = OracleConfig::new(
      hylo.oracle_interval_secs,
      convert_ufixvalue64(hylo.oracle_conf_tolerance).try_into()?,
    );
    let stability_controller = StabilityController::new(
      convert_ufixvalue64(hylo.stability_threshold_1).try_into()?,
      convert_ufixvalue64(hylo.stability_threshold_2).try_into()?,
    )?;
    Ok(HyloJupiterClient {
      clock: amm_context.clock_ref.clone(),
      total_sol_cache: hylo.total_sol_cache.into(),
      stability_controller,
      oracle_config,
      hyusd_fees: hylo.stablecoin_fees.into(),
      xsol_fees: hylo.levercoin_fees.into(),
      hyusd_mint: None,
      xsol_mint: None,
      shyusd_mint: None,
      jitosol_header: None,
      sol_usd: None,
      hyusd_pool: None,
      xsol_pool: None,
      pool_config: None,
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
    vec![HYUSD, XSOL, SHYUSD, JITOSOL]
  }

  fn get_accounts_to_update(&self) -> Vec<Pubkey> {
    vec![
      HYUSD,
      XSOL,
      pda::lst_header(JITOSOL),
      SOL_USD_PYTH_FEED,
      SHYUSD,
      *pda::HYUSD_POOL,
      *pda::XSOL_POOL,
      *pda::POOL_CONFIG,
    ]
  }

  fn update(&mut self, account_map: &AccountMap) -> Result<()> {
    let hyusd_mint: Mint = account_map_get(account_map, &HYUSD)?;
    let xsol_mint: Mint = account_map_get(account_map, &XSOL)?;
    let jitosol_header: LstHeader =
      account_map_get(account_map, &pda::lst_header(JITOSOL))?;
    let sol_usd: PriceUpdateV2 =
      account_map_get(account_map, &SOL_USD_PYTH_FEED)?;
    let shyusd_mint: Mint = account_map_get(account_map, &SHYUSD)?;
    let hyusd_pool: TokenAccount =
      account_map_get(account_map, &pda::HYUSD_POOL)?;
    let xsol_pool: TokenAccount =
      account_map_get(account_map, &pda::XSOL_POOL)?;
    let pool_config: PoolConfig =
      account_map_get(account_map, &pda::POOL_CONFIG)?;
    self.hyusd_mint = Some(hyusd_mint);
    self.xsol_mint = Some(xsol_mint);
    self.shyusd_mint = Some(shyusd_mint);
    self.jitosol_header = Some(jitosol_header);
    self.sol_usd = Some(sol_usd);
    self.hyusd_pool = Some(hyusd_pool);
    self.xsol_pool = Some(xsol_pool);
    self.pool_config = Some(pool_config);
    Ok(())
  }

  fn quote(
    &self,
    QuoteParams {
      amount,
      input_mint,
      output_mint,
      swap_mode: _,
    }: &QuoteParams,
  ) -> Result<Quote> {
    let ctx = self.load_exchange_ctx()?;
    match (*input_mint, *output_mint) {
      (JITOSOL, HYUSD) => {
        quote::hyusd_mint(&ctx, self.jitosol_header()?, UFix64::new(*amount))
      }
      (HYUSD, JITOSOL) => {
        quote::hyusd_redeem(&ctx, self.jitosol_header()?, UFix64::new(*amount))
      }
      (JITOSOL, XSOL) => {
        quote::xsol_mint(&ctx, self.jitosol_header()?, UFix64::new(*amount))
      }
      (XSOL, JITOSOL) => {
        quote::xsol_redeem(&ctx, self.jitosol_header()?, UFix64::new(*amount))
      }
      (HYUSD, XSOL) => quote::hyusd_xsol_swap(&ctx, UFix64::new(*amount)),
      (XSOL, HYUSD) => quote::xsol_hyusd_swap(&ctx, UFix64::new(*amount)),
      (HYUSD, SHYUSD) => quote::shyusd_mint(
        &ctx,
        self.shyusd_mint()?,
        self.hyusd_pool()?,
        self.xsol_pool()?,
        UFix64::new(*amount),
      ),
      (SHYUSD, HYUSD) => quote::shyusd_redeem(
        self.shyusd_mint()?,
        self.hyusd_pool()?,
        self.xsol_pool()?,
        self.pool_config()?,
        UFix64::new(*amount),
      ),
      (SHYUSD, JITOSOL) => quote::shyusd_redeem_lst(
        &ctx,
        self.shyusd_mint()?,
        self.hyusd_pool()?,
        self.xsol_pool()?,
        self.pool_config()?,
        self.jitosol_header()?,
        UFix64::new(*amount),
      ),
      _ => Err(anyhow!("Unsupported quote pair")),
    }
  }

  fn get_swap_and_account_metas(
    &self,
    _swap_params: &SwapParams,
  ) -> Result<SwapAndAccountMetas> {
    todo!()
  }

  fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
    Box::new(self.clone())
  }
}
