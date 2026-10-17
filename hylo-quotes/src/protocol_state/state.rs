//! Protocol state types and deserialization
//!
//! Contains the `ProtocolState` struct and its construction from protocol
//! accounts.

use std::collections::HashMap;

use anchor_client::solana_sdk::clock::{Clock, UnixTimestamp};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{ensure, Context, Result};
use fix::prelude::*;
use hylo_core::asset_swap_config::AssetSwapConfig;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::LstExchangeContext;
use hylo_core::fees::controller::LevercoinFees;
use hylo_core::idl::earn_pool::accounts::PoolConfig;
use hylo_core::idl::exchange::accounts::{Hylo, LstHeader, UsdcPair};
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::lst::total_sol_cache::TotalSolCache;
use hylo_core::par_tolerance::ParTolerance;
use hylo_core::pyth::{
  query_pyth_oracle, validate_publish_time, OracleConfig, ORACLE_DIVISOR,
};
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::reserve_gate::ReserveGate;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::virtual_stablecoin::VirtualStablecoin;
use hylo_idl::tokens::{Exo, TokenMint, HYLOSOL, JITOSOL};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::protocol_state::accounts::{decode, decode_clock};
use crate::protocol_state::{
  read_exo_entries, ExoAccounts, ExoPairState, ProtocolAccounts,
};
use crate::LST;

/// USDC exchange state for stablecoin mint/redeem.
#[derive(Clone)]
pub struct UsdcExchangeState {
  /// Fee extracted when minting stablecoin from USDC
  pub mint_fee: UFix64<N4>,
  /// Fee extracted when redeeming stablecoin to USDC
  pub redeem_fee: UFix64<N4>,
  /// USDC pair pause flag
  pub paused: bool,
  /// USDC collateral vault balance
  pub vault_balance: UFix64<N6>,
  /// Virtual stablecoin counter for the USDC pair
  pub virtual_stablecoin: VirtualStablecoin,
  /// USDC/USD spot price, or the oracle validation error that rejected it.
  /// A rejected oracle disables USDC routes only.
  pub usdc_usd_spot: Result<UFix64<N9>, CoreError>,
  /// Tolerated distance from par for the USDC pair
  pub par_tolerance: ParTolerance,
  /// Reserve policy for collateral-to-USDC rebalances.
  pub reserve_gate: ReserveGate,
}

/// Tests a feed publish time against the tightened stablecoin oracle window.
pub(crate) fn in_stablecoin_oracle_window(
  publish_time: i64,
  interval_secs: u64,
  now: i64,
) -> bool {
  validate_publish_time(
    publish_time,
    interval_secs.div_ceil(ORACLE_DIVISOR),
    now,
  )
  .is_ok()
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

  /// Registered Exo pairs keyed by collateral mint
  pub exo_pairs: HashMap<Pubkey, ExoPairState<C>>,

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

  /// SOL/USD oracle publish time
  pub sol_usd_publish_time: i64,

  /// Full oracle staleness interval
  pub oracle_interval_secs: u64,
}

impl<C: SolanaClock> ProtocolState<C> {
  /// Build `ProtocolState` from deserialized accounts and a clock.
  ///
  /// # Errors
  /// * Propagates errors from [`build_lst_exchange_context`].
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
    exo_accounts: &[ExoAccounts],
    usdc_exchange_state: UsdcExchangeState,
    jitosol_stake_pool: SplStakePool,
    hylosol_stake_pool: SplStakePool,
    jitosol_vault_balance: UFix64<N9>,
    hylosol_vault_balance: UFix64<N9>,
  ) -> Result<ProtocolState<C>>
  where
    C: Clone,
  {
    let exo_pairs = exo_accounts
      .iter()
      .map(|accounts| {
        let pair = accounts.pair_state(clock.clone())?;
        Ok((pair.collateral_mint, pair))
      })
      .collect::<Result<HashMap<_, _>>>()?;
    let sol_usd_publish_time = sol_usd.price_message.publish_time;
    let fetched_at = clock.unix_timestamp();
    let lst_swap_config = AssetSwapConfig::new(hylo.lst_swap_fee.into())?;
    let exchange_context =
      build_lst_exchange_context(clock, hylo, &xsol_mint, sol_usd)?;
    Ok(ProtocolState {
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
      exo_pairs,
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
      protocol_paused: hylo.protocol_paused,
      lst_pair_paused: hylo.lst_pair_paused,
      pool_drawdown: hylo.pool_drawdown.into(),
      yield_harvest_epoch: hylo.yield_harvest_cache.epoch,
      jitosol_vault_balance,
      hylosol_vault_balance,
      sol_usd_publish_time,
      oracle_interval_secs: hylo.oracle_interval_secs,
    })
  }

  /// Tests the SOL/USD feed against the stablecoin oracle window.
  #[must_use]
  pub fn sol_usd_in_stablecoin_oracle_window(&self) -> bool {
    in_stablecoin_oracle_window(
      self.sol_usd_publish_time,
      self.oracle_interval_secs,
      self.exchange_context.clock.unix_timestamp(),
    )
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

  /// Selects the pair state for a registered exo collateral.
  ///
  /// # Errors
  /// * Collateral has no registered pair in this snapshot
  pub fn exo_pair<E: Exo>(&self) -> Result<&ExoPairState<C>, CoreError> {
    self.exo_pair_by_mint(E::MINT)
  }

  /// Selects the pair state for a registered Exo collateral mint.
  ///
  /// # Errors
  /// * Collateral has no registered pair in this snapshot.
  pub fn exo_pair_by_mint(
    &self,
    collateral_mint: Pubkey,
  ) -> Result<&ExoPairState<C>, CoreError> {
    self
      .exo_pairs
      .get(&collateral_mint)
      .ok_or(CoreError::UnknownExoMint)
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

/// Builds USDC exchange state from protocol accounts.
///
/// # Errors
/// * Deserialization or oracle failure
fn build_usdc_exchange_state(
  clock: &Clock,
  accounts: &ProtocolAccounts,
) -> Result<UsdcExchangeState> {
  let usdc_pair: UsdcPair = decode(&accounts.usdc_pair, "UsdcPair")?;
  let usdc_usd: PriceUpdateV2 =
    decode(&accounts.usdc_usd_pyth, "USDC/USD Pyth feed")?;

  let oracle_config = OracleConfig::new(
    usdc_pair.oracle_interval_secs,
    usdc_pair.oracle_conf_tolerance.try_into()?,
  );
  let usdc_usd_spot = query_pyth_oracle(clock, &usdc_usd, oracle_config)
    .map(|oracle| oracle.spot);
  let usdc_vault: TokenAccount = decode(&accounts.usdc_vault, "USDC vault")?;

  let virtual_stablecoin: VirtualStablecoin =
    usdc_pair.virtual_stablecoin.into();

  let reserve_gate = ReserveGate::validated(
    usdc_pair.reserve_gate.reserve_per_tvl.into(),
    usdc_pair.reserve_gate.tvl_cap.into(),
  )?;

  Ok(UsdcExchangeState {
    mint_fee: usdc_pair.mint_fee.try_into()?,
    redeem_fee: usdc_pair.redeem_fee.try_into()?,
    paused: usdc_pair.paused,
    vault_balance: UFix64::new(usdc_vault.amount),
    virtual_stablecoin,
    usdc_usd_spot,
    par_tolerance: usdc_pair.par_tolerance.into(),
    reserve_gate,
  })
}

impl TryFrom<&ProtocolAccounts> for ProtocolState<Clock> {
  type Error = anyhow::Error;

  /// Build `ProtocolState` from protocol accounts
  ///
  /// # Errors
  /// Returns error if any account fails deserialization.
  fn try_from(accounts: &ProtocolAccounts) -> Result<Self> {
    let hylo: Hylo = decode(&accounts.hylo, "Hylo")?;
    let jitosol_header: LstHeader =
      decode(&accounts.jitosol_header, "JitoSOL header")?;
    let hylosol_header: LstHeader =
      decode(&accounts.hylosol_header, "hyloSOL header")?;
    let hyusd_mint: Mint = decode(&accounts.hyusd_mint, "HYUSD mint")?;
    let shyusd_mint: Mint = decode(&accounts.shyusd_mint, "SHYUSD mint")?;
    let xsol_mint: Mint = decode(&accounts.xsol_mint, "XSOL mint")?;
    let pool_config: PoolConfig = decode(&accounts.pool_config, "Pool config")?;
    let hyusd_pool: TokenAccount = decode(&accounts.hyusd_pool, "HYUSD pool")?;
    let sol_usd: PriceUpdateV2 =
      decode(&accounts.sol_usd_pyth, "SOL/USD Pyth feed")?;
    let clock = decode_clock(&accounts.clock)?;
    let usdc_exchange_state = build_usdc_exchange_state(&clock, accounts)?;
    let jitosol_stake_pool =
      SplStakePool::from_bytes(&accounts.jitosol_pool_state.data)?;
    let hylosol_stake_pool =
      SplStakePool::from_bytes(&accounts.hylosol_pool_state.data)?;
    let jitosol_vault: TokenAccount =
      decode(&accounts.jitosol_vault, "JitoSOL vault")?;
    let hylosol_vault: TokenAccount =
      decode(&accounts.hylosol_vault, "hyloSOL vault")?;
    let entries = read_exo_entries(&accounts.exo_registry.data)?;
    ensure!(
      entries.len() == accounts.exo_accounts.len(),
      "Exo registry has {} entries but {} pair account groups were fetched",
      entries.len(),
      accounts.exo_accounts.len(),
    );
    let exo_pairs = entries
      .iter()
      .zip(&accounts.exo_accounts)
      .map(|(entry, raw)| ExoAccounts::parse(entry, raw))
      .collect::<Result<Vec<_>>>()?;
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
      &exo_pairs,
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
      UFix64::new(jitosol_vault.amount),
      UFix64::new(hylosol_vault.amount),
    )
  }
}
