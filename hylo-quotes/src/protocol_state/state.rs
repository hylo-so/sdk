//! Protocol state types and deserialization
//!
//! Contains the `ProtocolState` struct and its construction from protocol
//! accounts.

use std::collections::HashMap;

use anchor_client::solana_sdk::clock::{Clock, UnixTimestamp};
use anchor_lang::prelude::Pubkey;
use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Context, Result};
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
use hylo_core::solana_clock::SolanaClock;
use hylo_core::virtual_stablecoin::VirtualStablecoin;
use hylo_idl::tokens::{Exo, TokenMint, HYLOSOL, JITOSOL};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::protocol_state::{
  exo_registry_entries, read_exo_registry, ExoAccounts, ExoPairState,
  ProtocolAccounts,
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
  /// USDC/USD spot price, gated against par
  pub usdc_usd_spot: UFix64<N9>,
  /// Tolerated distance from par for the USDC pair
  pub par_tolerance: ParTolerance,
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
    let exo_pairs =
      ProtocolState::exo_pairs_from_accounts(&clock, exo_accounts)?;
    ProtocolState::build_from_exo_pairs(
      clock,
      hylo,
      jitosol_header,
      hylosol_header,
      hyusd_mint,
      xsol_mint,
      shyusd_mint,
      pool_config,
      hyusd_pool,
      sol_usd,
      exo_pairs,
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
      jitosol_vault_balance,
      hylosol_vault_balance,
    )
  }

  /// Build a core protocol snapshot.
  ///
  /// # Errors
  ///
  /// Returns an error if the core exchange context or a required protocol
  /// conversion cannot be constructed from the supplied accounts.
  #[allow(clippy::too_many_arguments)]
  pub fn build_base(
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
    usdc_exchange_state: UsdcExchangeState,
    jitosol_stake_pool: SplStakePool,
    hylosol_stake_pool: SplStakePool,
    jitosol_vault_balance: UFix64<N9>,
    hylosol_vault_balance: UFix64<N9>,
  ) -> Result<ProtocolState<C>> {
    ProtocolState::build_from_exo_pairs(
      clock,
      hylo,
      jitosol_header,
      hylosol_header,
      hyusd_mint,
      xsol_mint,
      shyusd_mint,
      pool_config,
      hyusd_pool,
      sol_usd,
      HashMap::new(),
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
      jitosol_vault_balance,
      hylosol_vault_balance,
    )
  }

  /// Loads Exo pair state from parsed accounts, dropping any prior pairs.
  ///
  /// # Errors
  /// * Pair state load
  pub fn load_exo_pairs(
    &mut self,
    clock: &C,
    exo_accounts: &[ExoAccounts],
  ) -> Result<()>
  where
    C: Clone,
  {
    self.exo_pairs =
      ProtocolState::exo_pairs_from_accounts(clock, exo_accounts)?;
    Ok(())
  }

  fn exo_pairs_from_accounts(
    clock: &C,
    exo_accounts: &[ExoAccounts],
  ) -> Result<HashMap<Pubkey, ExoPairState<C>>>
  where
    C: Clone,
  {
    exo_accounts
      .iter()
      .map(|accounts| {
        let pair = accounts.pair_state(clock.clone())?;
        Ok((accounts.collateral_mint(), pair))
      })
      .collect()
  }

  #[allow(clippy::too_many_arguments)]
  fn build_from_exo_pairs(
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
    exo_pairs: HashMap<Pubkey, ExoPairState<C>>,
    usdc_exchange_state: UsdcExchangeState,
    jitosol_stake_pool: SplStakePool,
    hylosol_stake_pool: SplStakePool,
    jitosol_vault_balance: UFix64<N9>,
    hylosol_vault_balance: UFix64<N9>,
  ) -> Result<ProtocolState<C>> {
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
  let usdc_pair =
    UsdcPair::try_deserialize(&mut accounts.usdc_pair.data.as_slice())?;
  let usdc_usd =
    PriceUpdateV2::try_deserialize(&mut accounts.usdc_usd_pyth.data.as_slice())
      .context("USDC/USD Pyth deserialization")?;

  let oracle_config = OracleConfig::new(
    usdc_pair.oracle_interval_secs,
    usdc_pair.oracle_conf_tolerance.try_into()?,
  );
  let usdc_oracle = query_pyth_oracle(clock, &usdc_usd, oracle_config)?;
  let usdc_vault =
    TokenAccount::try_deserialize(&mut accounts.usdc_vault.data.as_slice())?;

  let virtual_stablecoin: VirtualStablecoin =
    usdc_pair.virtual_stablecoin.into();

  Ok(UsdcExchangeState {
    mint_fee: usdc_pair.mint_fee.try_into()?,
    redeem_fee: usdc_pair.redeem_fee.try_into()?,
    paused: usdc_pair.paused,
    vault_balance: UFix64::new(usdc_vault.amount),
    virtual_stablecoin,
    usdc_usd_spot: usdc_oracle.spot,
    par_tolerance: usdc_pair.par_tolerance.into(),
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
    let exo_registry = read_exo_registry(&accounts.exo_registry.data)?;
    let entries = exo_registry_entries(&exo_registry)?;
    anyhow::ensure!(
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
