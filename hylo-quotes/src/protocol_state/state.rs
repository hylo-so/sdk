//! Protocol state types and deserialization
//!
//! Contains the `ProtocolState` struct and its construction from protocol
//! accounts.

use std::sync::Arc;

use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::clock::{Clock, UnixTimestamp};
use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Context, Result};
use fix::prelude::*;
use hylo_core::asset_swap_config::AssetSwapConfig;
use hylo_core::conversion::UsdcStablecoinConversion;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::{ExoExchangeContext, LstExchangeContext};
use hylo_core::fees::controller::LevercoinFees;
use hylo_core::idl::earn_pool::accounts::PoolConfig;
use hylo_core::idl::exchange::accounts::{ExoPair, Hylo, LstHeader, UsdcPair};
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::lst::total_sol_cache::TotalSolCache;
use hylo_core::pyth::OracleConfig;
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::solana_clock::SolanaClock;
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
  pub swap_fee: UFix64<N4>,
  /// USDC pair pause flag
  pub paused: bool,
  /// USDC collateral vault balance
  pub vault_balance: UFix64<N6>,
  /// Virtual stablecoin supply for the USDC pair
  pub virtual_stablecoin_supply: UFix64<N6>,
}

impl UsdcExchangeState {
  /// Builds the USDC stablecoin conversion from stored price range.
  #[must_use]
  pub fn conversion(&self) -> UsdcStablecoinConversion {
    UsdcStablecoinConversion {
      usdc_usd_price: self.usdc_usd_price,
    }
  }
}

/// [`ExoPair`] state not carried by the exchange context.
#[derive(Clone)]
pub struct BtcPairState {
  pub paused: bool,
  pub pool_drawdown: PoolDrawdown,
  pub borrow_rate_harvest_epoch: u64,
  pub supply_floor: UFix64<N6>,
}

impl TryFrom<&ExoPair> for BtcPairState {
  type Error = anyhow::Error;

  fn try_from(exo_pair: &ExoPair) -> Result<BtcPairState> {
    Ok(BtcPairState {
      paused: exo_pair.paused,
      pool_drawdown: exo_pair.pool_drawdown.into(),
      borrow_rate_harvest_epoch: exo_pair.borrow_rate_harvest_cache.epoch,
      supply_floor: exo_pair.virtual_stablecoin_supply_floor.try_into()?,
    })
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

  /// Earn pool configuration
  pub pool_config: PoolConfig,

  /// HYUSD earn pool token account
  pub hyusd_pool: TokenAccount,

  /// Timestamp of when this state was fetched
  pub fetched_at: UnixTimestamp,

  /// LST swap configuration
  pub lst_swap_config: AssetSwapConfig,

  /// cbBTC exo exchange context
  pub cbbtc_exchange_context: Arc<ExoExchangeContext<C>>,

  /// USDC exchange state
  pub usdc_exchange_state: UsdcExchangeState,

  /// `JitoSOL` SPL stake pool
  pub jitosol_stake_pool: SplStakePool,

  /// `hyloSOL` SPL stake pool
  pub hylosol_stake_pool: SplStakePool,

  /// Protocol-wide pause flag
  pub protocol_paused: bool,

  /// LST pair pause flag
  pub lst_pair_paused: bool,

  /// Drawdown repayment ledger
  pub pool_drawdown: PoolDrawdown,

  /// Epoch of the last yield harvest
  pub yield_harvest_epoch: u64,

  /// `JitoSOL` collateral vault balance
  pub jitosol_vault_balance: UFix64<N9>,

  /// `hyloSOL` collateral vault balance
  pub hylosol_vault_balance: UFix64<N9>,

  /// BTC pair gate state
  pub btc_pair_state: BtcPairState,
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
    sol_usd: &PriceUpdateV2,
    cbbtc_exchange_context: Arc<ExoExchangeContext<C>>,
    usdc_exchange_state: UsdcExchangeState,
    jitosol_stake_pool: SplStakePool,
    hylosol_stake_pool: SplStakePool,
    jitosol_vault_balance: UFix64<N9>,
    hylosol_vault_balance: UFix64<N9>,
    btc_pair_state: BtcPairState,
  ) -> Result<Self> {
    let fetched_at = clock.unix_timestamp();
    let lst_swap_config = AssetSwapConfig::new(hylo.lst_swap_fee.into())?;
    let exchange_context =
      build_lst_exchange_context(clock, hylo, &xsol_mint, sol_usd)?;
    Ok(Self {
      exchange_context,
      jitosol_header,
      hylosol_header,
      hyusd_mint,
      xsol_mint,
      shyusd_mint,
      pool_config,
      hyusd_pool,
      fetched_at,
      lst_swap_config,
      cbbtc_exchange_context,
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
      protocol_paused: hylo.protocol_paused,
      lst_pair_paused: hylo.lst_pair_paused,
      pool_drawdown: hylo.pool_drawdown.into(),
      yield_harvest_epoch: hylo.yield_harvest_cache.epoch,
      jitosol_vault_balance,
      hylosol_vault_balance,
      btc_pair_state,
    })
  }

  /// Selects an [`LstHeader`] field given a token implementing [`LST`].
  ///
  /// # Errors
  /// * LST does not have a corresponding header field in this struct
  pub fn lst_header<L: LST>(&self) -> Result<&LstHeader, CoreError> {
    match L::MINT {
      JITOSOL::MINT => Ok(&self.jitosol_header),
      HYLOSOL::MINT => Ok(&self.hylosol_header),
      _ => Err(CoreError::UnknownLstMint),
    }
  }

  /// Collateral vault balance for the given LST.
  ///
  /// # Errors
  /// * Unknown LST mint
  pub fn lst_vault_balance<L: LST>(&self) -> Result<UFix64<N9>, CoreError> {
    match L::MINT {
      JITOSOL::MINT => Ok(self.jitosol_vault_balance),
      HYLOSOL::MINT => Ok(self.hylosol_vault_balance),
      _ => Err(CoreError::UnknownLstMint),
    }
  }

  /// SPL stake pool for the given LST.
  ///
  /// # Errors
  /// * Unknown LST mint
  pub fn stake_pool<L: LST>(&self) -> Result<&SplStakePool, CoreError> {
    match L::MINT {
      JITOSOL::MINT => Ok(&self.jitosol_stake_pool),
      HYLOSOL::MINT => Ok(&self.hylosol_stake_pool),
      _ => Err(CoreError::UnknownLstMint),
    }
  }

  #[must_use]
  pub fn cbbtc_exchange_context(&self) -> &ExoExchangeContext<C> {
    &self.cbbtc_exchange_context
  }

  #[must_use]
  pub fn usdc_exchange_state(&self) -> &UsdcExchangeState {
    &self.usdc_exchange_state
  }
}

/// Builds the `LstExchangeContext` from protocol accounts.
///
/// # Errors
/// * Oracle, curve, or stability controller validation
pub fn build_lst_exchange_context<C: SolanaClock>(
  clock: C,
  hylo: &Hylo,
  xsol_mint: &Mint,
  sol_usd: &PriceUpdateV2,
) -> Result<LstExchangeContext<C>> {
  let total_sol_cache: TotalSolCache = hylo.total_sol_cache.into();
  let oracle_config = OracleConfig::new(
    hylo.oracle_interval_secs,
    hylo.oracle_conf_tolerance.try_into()?,
  );
  let xsol_fees: LevercoinFees = hylo.levercoin_fees.into();
  LstExchangeContext::load(
    clock,
    &total_sol_cache,
    hylo.stablecoin_mint_threshold.try_into()?,
    oracle_config,
    xsol_fees,
    sol_usd,
    hylo.virtual_stablecoin.into(),
    Some(xsol_mint),
    hylo.lst_sell_curve_config.into(),
    hylo.lst_buy_curve_config.into(),
  )
  .context("LstExchangeContext::load")
}

/// Builds the cbBTC `ExoExchangeContext` from protocol accounts.
///
/// # Errors
/// * Deserialization or context-load failure
pub fn build_cbbtc_exchange_context(
  clock: Clock,
  exo_pair: &Account,
  vault: &Account,
  xbtc_mint: &Account,
  btc_usd: &Account,
) -> Result<ExoExchangeContext<Clock>> {
  let exo_pair = ExoPair::try_deserialize(&mut exo_pair.data.as_slice())?;
  let vault = TokenAccount::try_deserialize(&mut vault.data.as_slice())?;
  let xbtc_mint = Mint::try_deserialize(&mut xbtc_mint.data.as_slice())?;
  let btc_usd = PriceUpdateV2::try_deserialize(&mut btc_usd.data.as_slice())
    .context("BTC/USD Pyth deserialization")?;

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

  ExoExchangeContext::load(
    clock,
    total_collateral,
    exo_pair.stablecoin_mint_threshold.try_into()?,
    oracle_config,
    levercoin_fees,
    &btc_usd,
    virtual_stablecoin,
    Some(&xbtc_mint),
    exo_pair.sell_curve_config.into(),
    exo_pair.buy_curve_config.into(),
    exo_pair.levercoin_market_cap_limit.try_into()?,
  )
  .context("ExoExchangeContext::load")
}

/// Builds USDC exchange state from protocol accounts.
///
/// # Errors
/// * Deserialization or oracle failure
fn build_usdc_exchange_state(
  clock: &Clock,
  accounts: &ProtocolAccounts,
) -> Result<UsdcExchangeState> {
  let usdc_pair =
    UsdcPair::try_deserialize(&mut accounts.usdc_pair.data.as_slice())?;
  let usdc_usd =
    PriceUpdateV2::try_deserialize(&mut accounts.usdc_usd_pyth.data.as_slice())
      .context("USDC/USD Pyth deserialization")?;

  let oracle_config = OracleConfig::new(
    usdc_pair.oracle_interval_secs,
    usdc_pair.oracle_conf_tolerance.try_into()?,
  );
  let usdc_oracle =
    hylo_core::pyth::query_pyth_oracle(clock, &usdc_usd, oracle_config)?;
  let usdc_usd_price = usdc_oracle.price_range()?;
  let usdc_vault =
    TokenAccount::try_deserialize(&mut accounts.usdc_vault.data.as_slice())?;

  let virtual_stablecoin: VirtualStablecoin =
    usdc_pair.virtual_stablecoin.into();

  Ok(UsdcExchangeState {
    usdc_usd_price,
    swap_fee: usdc_pair.swap_fee.try_into()?,
    paused: usdc_pair.paused,
    vault_balance: UFix64::new(usdc_vault.amount),
    virtual_stablecoin_supply: virtual_stablecoin.supply()?,
  })
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

    let sol_usd = PriceUpdateV2::try_deserialize(
      &mut accounts.sol_usd_pyth.data.as_slice(),
    )
    .context("SOL/USD Pyth deserialization")?;

    let clock: Clock = bincode::deserialize(&accounts.clock.data)
      .map_err(|e| anyhow!("Failed to deserialize clock: {e}"))?;

    let cbbtc_exchange_context = Arc::new(build_cbbtc_exchange_context(
      clock.clone(),
      &accounts.cbbtc_exo_pair,
      &accounts.cbbtc_vault,
      &accounts.xbtc_mint,
      &accounts.btc_usd_pyth,
    )?);
    let usdc_exchange_state = build_usdc_exchange_state(&clock, accounts)?;

    let jitosol_stake_pool =
      SplStakePool::from_bytes(&accounts.jitosol_pool_state.data)?;
    let hylosol_stake_pool =
      SplStakePool::from_bytes(&accounts.hylosol_pool_state.data)?;

    let jitosol_vault = TokenAccount::try_deserialize(
      &mut accounts.jitosol_vault.data.as_slice(),
    )?;
    let hylosol_vault = TokenAccount::try_deserialize(
      &mut accounts.hylosol_vault.data.as_slice(),
    )?;
    let exo_pair =
      ExoPair::try_deserialize(&mut accounts.cbbtc_exo_pair.data.as_slice())?;
    let btc_pair_state = BtcPairState::try_from(&exo_pair)?;

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
      &sol_usd,
      cbbtc_exchange_context,
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
      UFix64::new(jitosol_vault.amount),
      UFix64::new(hylosol_vault.amount),
      btc_pair_state,
    )
  }
}
