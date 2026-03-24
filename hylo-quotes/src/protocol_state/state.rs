//! Protocol state types and deserialization
//!
//! Contains the `ProtocolState` struct and its construction from protocol
//! accounts.

use std::sync::Arc;

use anchor_client::solana_sdk::clock::{Clock, UnixTimestamp};
use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Result};
use fix::prelude::*;
use hylo_core::asset_swap_config::AssetSwapConfig;
use hylo_core::conversion::UsdcStablecoinConversion;
use hylo_core::exchange_context::{ExoExchangeContext, LstExchangeContext};
use hylo_core::fee_controller::{FeeExtract, LevercoinFees};
use hylo_core::idl::exchange::accounts::{ExoPair, Hylo, LstHeader, UsdcPair};
use hylo_core::idl::stability_pool::accounts::PoolConfig;
use hylo_core::pyth::OracleConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::total_sol_cache::TotalSolCache;
use hylo_core::virtual_stablecoin::VirtualStablecoin;
use hylo_idl::tokens::{TokenMint, HYLOSOL, JITOSOL};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::protocol_state::ProtocolAccounts;
use crate::LST;

/// USDC exchange state for stablecoin mint/redeem.
#[derive(Clone)]
pub struct UsdcExchangeState {
  /// USDC/USD oracle price range
  pub usdc_usd_price: hylo_core::pyth::PriceRange<N9>,
  /// Swap fee extracted on USDC operations
  pub swap_fee: UFix64<N9>,
}

impl UsdcExchangeState {
  /// Builds the USDC stablecoin conversion from stored price range.
  #[must_use]
  pub fn conversion(&self) -> UsdcStablecoinConversion {
    UsdcStablecoinConversion {
      usdc_usd_price: self.usdc_usd_price,
    }
  }

  /// Applies the swap fee to a USDC amount.
  ///
  /// # Errors
  /// * Arithmetic failure in fee extraction
  pub fn apply_fee(
    &self,
    amount: UFix64<N6>,
  ) -> Result<FeeExtract<N6>> {
    Ok(FeeExtract::new(self.swap_fee, amount)?)
  }
}

/// Complete snapshot of Hylo protocol state
#[derive(Clone)]
pub struct ProtocolState<C: SolanaClock> {
  /// Exchange context with all protocol parameters
  pub exchange_context: LstExchangeContext<C>,

  /// `JitoSOL` LST header
  pub jitosol_header: LstHeader,

  /// `HyloSOL` LST header
  pub hylosol_header: LstHeader,

  /// HYUSD mint account
  pub hyusd_mint: Mint,

  /// XSOL mint account
  pub xsol_mint: Mint,

  /// SHYUSD mint account
  pub shyusd_mint: Mint,

  /// Stability pool configuration
  pub pool_config: PoolConfig,

  /// HYUSD stability pool token account
  pub hyusd_pool: TokenAccount,

  /// XSOL stability pool token account
  pub xsol_pool: TokenAccount,

  /// Timestamp of when this state was fetched
  pub fetched_at: UnixTimestamp,

  /// LST swap configuration
  pub lst_swap_config: AssetSwapConfig,

  /// cbBTC exo exchange context (None if not initialized)
  pub cbbtc_exo_context: Option<Arc<ExoExchangeContext<C>>>,

  /// USDC exchange state (None if not initialized)
  pub usdc_exchange_state: Option<UsdcExchangeState>,
}

impl<C: SolanaClock> ProtocolState<C> {
  /// Build `ProtocolState` from deserialized accounts and a clock.
  ///
  /// # Errors
  /// * Propagates errors from `ExchangeContext::load`.
  #[allow(clippy::too_many_arguments)]
  pub fn build(
    clock: C,
    hylo: &Hylo,
    jitosol_header: LstHeader,
    hylosol_header: LstHeader,
    hyusd_mint: Mint,
    xsol_mint: Mint,
    shyusd_mint: Mint,
    pool_config: PoolConfig,
    hyusd_pool: TokenAccount,
    xsol_pool: TokenAccount,
    sol_usd: &PriceUpdateV2,
    cbbtc_exo_context: Option<Arc<ExoExchangeContext<C>>>,
    usdc_exchange_state: Option<UsdcExchangeState>,
  ) -> Result<Self> {
    let fetched_at = clock.unix_timestamp();
    let total_sol_cache: TotalSolCache = hylo.total_sol_cache.into();
    let oracle_config = OracleConfig::new(
      hylo.oracle_interval_secs,
      hylo.oracle_conf_tolerance.try_into()?,
    );
    let xsol_fees: LevercoinFees = hylo.levercoin_fees.into();
    let lst_swap_config = AssetSwapConfig::new(hylo.lst_swap_fee.into())?;
    let exchange_context = LstExchangeContext::load(
      clock,
      &total_sol_cache,
      hylo.stability_threshold_1.try_into()?,
      oracle_config,
      xsol_fees,
      sol_usd,
      hylo.virtual_stablecoin.into(),
      Some(&xsol_mint),
      hylo.lst_sell_curve_config.into(),
      hylo.lst_buy_curve_config.into(),
    )?;
    Ok(Self {
      exchange_context,
      jitosol_header,
      hylosol_header,
      hyusd_mint,
      xsol_mint,
      shyusd_mint,
      pool_config,
      hyusd_pool,
      xsol_pool,
      fetched_at,
      lst_swap_config,
      cbbtc_exo_context,
      usdc_exchange_state,
    })
  }

  /// Selects an [`LstHeader`] field given a token implementing [`LST`].
  ///
  /// # Errors
  /// * LST does not have a corresponding header field in this struct
  pub fn lst_header<L: LST>(&self) -> Result<&LstHeader> {
    match L::MINT {
      JITOSOL::MINT => Ok(&self.jitosol_header),
      HYLOSOL::MINT => Ok(&self.hylosol_header),
      _ => Err(anyhow!("LstHeader not found for {}", L::MINT)),
    }
  }

  /// Returns the cbBTC `ExoExchangeContext`, if available.
  ///
  /// # Errors
  /// * cbBTC exo pair not initialized
  pub fn cbbtc_exo_context(
    &self,
  ) -> Result<&ExoExchangeContext<C>> {
    self
      .cbbtc_exo_context
      .as_deref()
      .ok_or_else(|| anyhow!("cbBTC exo pair not initialized"))
  }

  /// Returns the USDC exchange state, if available.
  ///
  /// # Errors
  /// * USDC pair not initialized
  pub fn usdc_exchange_state(
    &self,
  ) -> Result<&UsdcExchangeState> {
    self
      .usdc_exchange_state
      .as_ref()
      .ok_or_else(|| anyhow!("USDC pair not initialized"))
  }
}

/// Attempts to build a cbBTC `ExoExchangeContext` from optional accounts.
///
/// Returns `None` if any required account is missing.
///
/// # Errors
/// * Deserialization or context-load failure when accounts are present
fn try_build_cbbtc_exo_context(
  clock: Clock,
  accounts: &ProtocolAccounts,
) -> Result<Option<ExoExchangeContext<Clock>>> {
  let (Some(pair_acct), Some(vault_acct), Some(xbtc_acct), Some(pyth_acct)) = (
    accounts.cbbtc_exo_pair.as_ref(),
    accounts.cbbtc_vault.as_ref(),
    accounts.xbtc_mint.as_ref(),
    accounts.btc_usd_pyth.as_ref(),
  ) else {
    return Ok(None);
  };

  let exo_pair =
    ExoPair::try_deserialize(&mut pair_acct.data.as_slice())?;
  let vault = TokenAccount::try_deserialize(
    &mut vault_acct.data.as_slice(),
  )?;
  let xbtc_mint =
    Mint::try_deserialize(&mut xbtc_acct.data.as_slice())?;
  let btc_usd = PriceUpdateV2::try_deserialize(
    &mut pyth_acct.data.as_slice(),
  )
  .map_err(|e| anyhow!("Failed to deserialize BTC/USD Pyth: {e}"))?;

  let oracle_config = OracleConfig::new(
    exo_pair.oracle_interval_secs,
    exo_pair.oracle_conf_tolerance.try_into()?,
  );
  let virtual_stablecoin: VirtualStablecoin =
    exo_pair.virtual_stablecoin.into();
  let levercoin_fees: LevercoinFees = exo_pair.levercoin_fees.into();
  let total_collateral: UFix64<N9> = UFix64::<N8>::new(vault.amount)
    .checked_convert()
    .ok_or_else(|| anyhow!("cbBTC vault amount N8->N9 overflow"))?;

  let ctx = ExoExchangeContext::load(
    clock,
    total_collateral,
    exo_pair.stability_threshold_1.try_into()?,
    oracle_config,
    levercoin_fees,
    &btc_usd,
    virtual_stablecoin,
    Some(&xbtc_mint),
    exo_pair.sell_curve_config.into(),
    exo_pair.buy_curve_config.into(),
  )?;
  Ok(Some(ctx))
}

/// Attempts to build USDC exchange state from optional accounts.
///
/// Returns `None` if any required account is missing.
///
/// # Errors
/// * Deserialization or oracle failure when accounts are present
fn try_build_usdc_exchange_state(
  clock: &Clock,
  accounts: &ProtocolAccounts,
) -> Result<Option<UsdcExchangeState>> {
  let (Some(pair_acct), Some(pyth_acct)) = (
    accounts.usdc_pair.as_ref(),
    accounts.usdc_usd_pyth.as_ref(),
  ) else {
    return Ok(None);
  };

  let usdc_pair =
    UsdcPair::try_deserialize(&mut pair_acct.data.as_slice())?;
  let usdc_usd = PriceUpdateV2::try_deserialize(
    &mut pyth_acct.data.as_slice(),
  )
  .map_err(|e| anyhow!("Failed to deserialize USDC/USD Pyth: {e}"))?;

  let oracle_config = OracleConfig::new(
    usdc_pair.oracle_interval_secs,
    usdc_pair.oracle_conf_tolerance.try_into()?,
  );
  let usdc_oracle =
    hylo_core::pyth::query_pyth_oracle(clock, &usdc_usd, oracle_config)?;
  let usdc_usd_price = usdc_oracle.price_range()?;

  Ok(Some(UsdcExchangeState {
    usdc_usd_price,
    swap_fee: usdc_pair.swap_fee.try_into()?,
  }))
}

impl TryFrom<&ProtocolAccounts> for ProtocolState<Clock> {
  type Error = anyhow::Error;

  /// Build `ProtocolState` from protocol accounts
  ///
  /// # Errors
  /// Returns error if any account fails deserialization.
  fn try_from(accounts: &ProtocolAccounts) -> Result<Self> {
    let hylo = Hylo::try_deserialize(&mut accounts.hylo.data.as_slice())?;

    let jitosol_header =
      LstHeader::try_deserialize(&mut accounts.jitosol_header.data.as_slice())?;

    let hylosol_header =
      LstHeader::try_deserialize(&mut accounts.hylosol_header.data.as_slice())?;

    let hyusd_mint =
      Mint::try_deserialize(&mut accounts.hyusd_mint.data.as_slice())?;

    let shyusd_mint =
      Mint::try_deserialize(&mut accounts.shyusd_mint.data.as_slice())?;

    let xsol_mint =
      Mint::try_deserialize(&mut accounts.xsol_mint.data.as_slice())?;

    let pool_config =
      PoolConfig::try_deserialize(&mut accounts.pool_config.data.as_slice())?;

    let hyusd_pool =
      TokenAccount::try_deserialize(&mut accounts.hyusd_pool.data.as_slice())?;

    let xsol_pool =
      TokenAccount::try_deserialize(&mut accounts.xsol_pool.data.as_slice())?;

    let sol_usd = PriceUpdateV2::try_deserialize(
      &mut accounts.sol_usd_pyth.data.as_slice(),
    )
    .map_err(|e| anyhow!("Failed to deserialize Pyth: {e}"))?;

    let clock: Clock = bincode::deserialize(&accounts.clock.data)
      .map_err(|e| anyhow!("Failed to deserialize clock: {e}"))?;

    let cbbtc_exo_context =
      try_build_cbbtc_exo_context(clock.clone(), accounts)?
        .map(Arc::new);
    let usdc_exchange_state =
      try_build_usdc_exchange_state(&clock, accounts)?;

    Self::build(
      clock,
      &hylo,
      jitosol_header,
      hylosol_header,
      hyusd_mint,
      xsol_mint,
      shyusd_mint,
      pool_config,
      hyusd_pool,
      xsol_pool,
      &sol_usd,
      cbbtc_exo_context,
      usdc_exchange_state,
    )
  }
}
