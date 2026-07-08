//! Earn pool yield statistics for sHYUSD: realized last-epoch yield,
//! naive APY, and projected next-epoch yield.

use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar;
use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::Result;
use fix::prelude::*;
use hylo_core::borrow_rate::BorrowRateConfig;
use hylo_core::earn_pool_math::lp_token_nav;
use hylo_core::exchange_context::{ExchangeContext, ExoExchangeContext};
use hylo_core::idl::exchange::accounts::{ExoPair, Hylo, LstHeader};
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::pyth::{query_pyth_oracle, OracleConfig};
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::yields::{HarvestCache, YieldHarvestConfig};
use hylo_idl::pda;
use hylo_idl::tokens::{StakePool, TokenMint, CBBTC, HYLOSOL, JITOSOL, SHYUSD};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::earn_pool_yield_math::{
  apply_drawdown_offset, epoch_yield_rate, lst_epoch_growth,
  projected_borrow_inflow, projected_lst_inflow, EPOCHS_PER_YEAR,
};
use crate::error::StatsError::{
  AccountCountMismatch, ClockDeserialize, ExoVaultConversion,
  LstVaultValueOverflow, MissingAccounts, NoBlockAtOrAfterSlot,
  NoPreviousEpoch, NonPositiveEpochDuration, PoolDrawdownOverflow,
  ProjectedExoInflowOverflow, ProjectedInflowOverflow,
  ProjectedLstInflowOverflow, RealizedYieldOverflow,
};

/// Seconds in a Julian year.
const SECONDS_PER_YEAR: f64 = 31_557_600.0;

/// Number of accounts fetched for [`EarnPoolStats`].
pub const STATS_ACCOUNT_COUNT: usize = 15;

/// Account keys required for [`EarnPoolStats`], in fetch order —
/// the same order [`build_stats_inputs`] destructures.
pub const STATS_ACCOUNT_KEYS: [Pubkey; STATS_ACCOUNT_COUNT] = [
  pda::HYLO,
  pda::lst_header(JITOSOL::MINT),
  pda::lst_header(HYLOSOL::MINT),
  pda::lst_vault(JITOSOL::MINT),
  pda::lst_vault(HYLOSOL::MINT),
  JITOSOL::POOL_STATE,
  HYLOSOL::POOL_STATE,
  pda::HYUSD_POOL,
  SHYUSD::MINT,
  pda::exo_pair(CBBTC::MINT),
  pda::exo_vault(CBBTC::MINT),
  pda::exo_levercoin_mint(CBBTC::MINT),
  pda::BTC_USD_PYTH_FEED,
  pda::SOL_USD_PYTH_FEED,
  sysvar::clock::ID,
];

/// Snapshot of one harvest stream from its on-chain [`HarvestCache`]:
/// the most recent harvest's epoch, the hyUSD it deposited into the
/// pool, and staleness (no harvest yet for the current epoch).
#[derive(Debug, Clone, Copy)]
pub struct RealizedHarvest {
  pub epoch: u64,
  pub hyusd_to_pool: UFix64<N6>,
  pub is_stale: bool,
}

/// One LST's projection inputs: vault SOL value and per-epoch growth.
#[derive(Debug, Clone, Copy)]
pub struct LstPosition {
  pub sol_value: UFix64<N9>,
  pub epoch_growth: UFix64<N9>,
}

/// One exogenous-collateral pair's inputs for the borrow-rate stream,
/// labeled by its collateral mint.
#[derive(Debug, Clone, Copy)]
pub struct ExoStream {
  pub collateral_mint: Pubkey,
  pub harvest_cache: HarvestCache,
  pub borrow_rate_config: BorrowRateConfig,
  pub levercoin_market_cap: UFix64<N9>,
}

/// Deserialized on-chain inputs for [`compute_stats`].
#[derive(Debug, Clone)]
pub struct StatsInputs {
  pub current_epoch: u64,
  pub pool_balance: UFix64<N6>,
  pub shyusd_supply: UFix64<N6>,
  pub lst_harvest_cache: HarvestCache,
  pub harvest_config: YieldHarvestConfig,
  pub lst_positions: Vec<LstPosition>,
  pub exo_streams: Vec<ExoStream>,
  pub sol_usd_spot: UFix64<N9>,
  pub outstanding_drawdown: UFix64<N6>,
  pub epochs_per_year: f64,
}

/// Per-stream results for one exo borrow-rate stream, labeled by its
/// collateral mint: realized harvest snapshot and projected next-epoch
/// hyUSD inflow.
#[derive(Debug, Clone, Copy)]
pub struct ExoStreamStats {
  pub collateral_mint: Pubkey,
  pub harvest: RealizedHarvest,
  pub projected_inflow: UFix64<N6>,
}

/// Earn pool yield statistics for sHYUSD.
///
/// * `nav` — hyUSD per sHYUSD
/// * `pool_balance` — current hyUSD in the pool, the denominator for both yield
///   rates
/// * `epochs_per_year` — annualization basis used for both APYs
/// * `lst_harvest` — LST staking-yield stream (`harvest_yield`); `exo_streams`
///   — exo borrow-rate streams (`harvest_borrow_rate`), one per pair
/// * `last_epoch_yield_rate` — sum of the LST stream plus all exo streams at
///   the most recent harvested epoch, over the pool; `naive_apy` — `(1 +
///   last_epoch_yield_rate)^epochs_per_year - 1`
/// * `projected_epoch_rate` — net projected inflow next epoch (LST + exo, minus
///   outstanding drawdown) over the pool; `projected_apy` — its annualization
#[derive(Debug, Clone)]
pub struct EarnPoolStats {
  pub nav: UFix64<N6>,
  pub pool_balance: UFix64<N6>,
  pub shyusd_supply: UFix64<N6>,
  pub current_epoch: u64,
  pub epochs_per_year: f64,
  pub lst_harvest: RealizedHarvest,
  pub exo_streams: Vec<ExoStreamStats>,
  pub last_epoch_yield_rate: UFix64<N9>,
  pub naive_apy: f64,
  pub projected_lst_inflow: UFix64<N6>,
  pub projected_exo_inflow: UFix64<N6>,
  pub outstanding_drawdown: UFix64<N6>,
  pub projected_epoch_rate: UFix64<N9>,
  pub projected_apy: f64,
}

/// Compounded APY from an `N9` per-epoch rate at a given annualization
/// basis (epochs per year).
#[allow(clippy::cast_precision_loss)] // advisory stats; f64 suffices
#[must_use]
pub fn annualize_with(per_epoch_rate: UFix64<N9>, epochs_per_year: f64) -> f64 {
  let rate = per_epoch_rate.bits as f64 * 1e-9;
  (1.0 + rate).powf(epochs_per_year) - 1.0
}

/// Compounded APY at the protocol's 182-epochs/year convention.
#[allow(clippy::cast_precision_loss)] // advisory stats; f64 suffices
#[must_use]
pub fn annualize(per_epoch_rate: UFix64<N9>) -> f64 {
  annualize_with(per_epoch_rate, EPOCHS_PER_YEAR as f64)
}

/// Measures the last completed epoch's exact wall-clock duration from
/// block times at the epoch boundary slots, returning epochs per year.
///
/// # Errors
/// * RPC failure, missing boundary blocks, or non-positive duration
#[allow(clippy::cast_precision_loss)] // advisory stats
pub async fn measure_epochs_per_year(
  rpc: &RpcClient,
  current_epoch: u64,
) -> Result<f64> {
  let prev_epoch = current_epoch.checked_sub(1).ok_or(NoPreviousEpoch)?;
  let schedule = rpc.get_epoch_schedule().await?;
  let start_prev = schedule.get_first_slot_in_epoch(prev_epoch);
  let start_curr = schedule.get_first_slot_in_epoch(current_epoch);
  let t0 = block_time_at_or_after(rpc, start_prev).await?;
  let t1 = block_time_at_or_after(rpc, start_curr).await?;
  let duration = t1
    .checked_sub(t0)
    .filter(|d| *d > 0)
    .ok_or(NonPositiveEpochDuration)?;
  Ok(SECONDS_PER_YEAR / duration as f64)
}

/// Block time of the first block at or after `slot` (epoch boundary
/// slots can be skipped).
async fn block_time_at_or_after(rpc: &RpcClient, slot: u64) -> Result<i64> {
  let slots = rpc.get_blocks_with_limit(slot, 1).await?;
  let first = slots.first().copied().ok_or(NoBlockAtOrAfterSlot(slot))?;
  Ok(rpc.get_block_time(first).await?)
}

fn realized(
  harvest_cache: &HarvestCache,
  current_epoch: u64,
) -> Result<RealizedHarvest> {
  Ok(RealizedHarvest {
    epoch: harvest_cache.epoch,
    hyusd_to_pool: harvest_cache.stablecoin_to_pool.try_into()?,
    is_stale: harvest_cache.is_stale(current_epoch),
  })
}

/// Computes yield statistics from deserialized on-chain inputs.
///
/// # Errors
/// * Arithmetic overflow in yield math
/// * Invalid fixed-point data in harvest caches or configs
pub fn compute_stats(inputs: &StatsInputs) -> Result<EarnPoolStats> {
  let nav = lp_token_nav(inputs.pool_balance, inputs.shyusd_supply)?;
  let lst_harvest = realized(&inputs.lst_harvest_cache, inputs.current_epoch)?;
  let exo_streams = inputs
    .exo_streams
    .iter()
    .map(|stream| {
      Ok(ExoStreamStats {
        collateral_mint: stream.collateral_mint,
        harvest: realized(&stream.harvest_cache, inputs.current_epoch)?,
        projected_inflow: projected_borrow_inflow(
          stream.levercoin_market_cap,
          &stream.borrow_rate_config,
        )?,
      })
    })
    .collect::<Result<Vec<ExoStreamStats>>>()?;

  let last_harvest_epoch = exo_streams
    .iter()
    .map(|stream| stream.harvest.epoch)
    .fold(lst_harvest.epoch, u64::max);
  let realized_total = std::iter::once(&lst_harvest)
    .chain(exo_streams.iter().map(|stream| &stream.harvest))
    .filter(|harvest| harvest.epoch == last_harvest_epoch)
    .try_fold(UFix64::zero(), |acc: UFix64<N6>, harvest| {
      acc.checked_add(&harvest.hyusd_to_pool)
    })
    .ok_or(RealizedYieldOverflow)?;
  let last_epoch_yield_rate =
    epoch_yield_rate(realized_total, inputs.pool_balance)?;

  let projected_lst = inputs.lst_positions.iter().try_fold(
    UFix64::zero(),
    |acc: UFix64<N6>, position| -> Result<UFix64<N6>> {
      let inflow = projected_lst_inflow(
        position.sol_value,
        position.epoch_growth,
        inputs.sol_usd_spot,
        &inputs.harvest_config,
      )?;
      Ok(acc.checked_add(&inflow).ok_or(ProjectedLstInflowOverflow)?)
    },
  )?;
  let projected_exo_inflow = exo_streams
    .iter()
    .try_fold(UFix64::zero(), |acc: UFix64<N6>, stream| {
      acc.checked_add(&stream.projected_inflow)
    })
    .ok_or(ProjectedExoInflowOverflow)?;
  let gross = projected_lst
    .checked_add(&projected_exo_inflow)
    .ok_or(ProjectedInflowOverflow)?;
  let net = apply_drawdown_offset(gross, inputs.outstanding_drawdown);
  let projected_epoch_rate = epoch_yield_rate(net, inputs.pool_balance)?;

  Ok(EarnPoolStats {
    nav,
    pool_balance: inputs.pool_balance,
    shyusd_supply: inputs.shyusd_supply,
    current_epoch: inputs.current_epoch,
    epochs_per_year: inputs.epochs_per_year,
    lst_harvest,
    exo_streams,
    last_epoch_yield_rate,
    naive_apy: annualize_with(last_epoch_yield_rate, inputs.epochs_per_year),
    projected_lst_inflow: projected_lst,
    projected_exo_inflow,
    outstanding_drawdown: inputs.outstanding_drawdown,
    projected_epoch_rate,
    projected_apy: annualize_with(projected_epoch_rate, inputs.epochs_per_year),
  })
}

/// Fetches [`EarnPoolStats`] from current on-chain state: one
/// slot-consistent `get_multiple_accounts` call, plus an epoch-schedule
/// fetch and two epoch-boundary block-time lookups to measure the last
/// completed epoch's duration. Read-only: needs no keypair or program
/// client.
///
/// # Errors
/// * RPC fetch, deserialization, or oracle validation failure
/// * Epoch duration measurement failure
/// * Arithmetic overflow in yield math
pub async fn fetch_earn_pool_stats(rpc: &RpcClient) -> Result<EarnPoolStats> {
  let fetched = rpc.get_multiple_accounts(&STATS_ACCOUNT_KEYS).await?;
  let accounts = resolve_stats_accounts(fetched)?;
  let clock: Clock =
    bincode::deserialize(&accounts[STATS_ACCOUNT_COUNT - 1].data)
      .map_err(ClockDeserialize)?;
  let epochs_per_year = measure_epochs_per_year(rpc, clock.epoch).await?;
  compute_stats(&build_stats_inputs(&accounts, epochs_per_year)?)
}

/// Resolves a fetched account list, erroring with the keys of any
/// missing accounts.
fn resolve_stats_accounts(
  accounts: Vec<Option<Account>>,
) -> Result<[Account; STATS_ACCOUNT_COUNT]> {
  let missing = STATS_ACCOUNT_KEYS
    .iter()
    .zip(&accounts)
    .filter(|(_, account)| account.is_none())
    .map(|(key, _)| *key)
    .collect::<Vec<Pubkey>>();
  if missing.is_empty() {
    Ok(
      accounts
        .into_iter()
        .flatten()
        .collect::<Vec<Account>>()
        .try_into()
        .map_err(|accounts: Vec<Account>| AccountCountMismatch {
          expected: STATS_ACCOUNT_COUNT,
          actual: accounts.len(),
        })?,
    )
  } else {
    Err(MissingAccounts(missing).into())
  }
}

/// Values the xBTC market cap for the borrow-rate projection.
/// Mirrors hylo-quotes `build_cbbtc_exchange_context`.
fn exo_levercoin_market_cap(
  clock: &Clock,
  exo_pair: &ExoPair,
  exo_vault: &TokenAccount,
  xbtc_mint: &Mint,
  btc_usd: &PriceUpdateV2,
) -> Result<UFix64<N9>> {
  let oracle_config = OracleConfig::new(
    exo_pair.oracle_interval_secs,
    exo_pair.oracle_conf_tolerance.try_into()?,
  );
  let total_collateral: UFix64<N9> = UFix64::<N8>::new(exo_vault.amount)
    .checked_convert()
    .ok_or(ExoVaultConversion)?;
  let exo_context = ExoExchangeContext::load(
    clock.clone(),
    total_collateral,
    exo_pair.stablecoin_mint_threshold.try_into()?,
    oracle_config,
    exo_pair.levercoin_fees.into(),
    btc_usd,
    exo_pair.virtual_stablecoin.into(),
    Some(xbtc_mint),
    exo_pair.sell_curve_config.into(),
    exo_pair.buy_curve_config.into(),
    exo_pair.levercoin_market_cap_limit.try_into()?,
  )?;
  let market_cap = exo_context.levercoin_market_cap()?;
  Ok(market_cap)
}

/// Sums outstanding pool drawdown across the LST pair and cbBTC exo pair.
fn total_outstanding_drawdown(
  hylo: &Hylo,
  exo_pair: &ExoPair,
) -> Result<UFix64<N6>> {
  let hylo_drawdown: PoolDrawdown = hylo.pool_drawdown.into();
  let exo_drawdown: PoolDrawdown = exo_pair.pool_drawdown.into();
  Ok(
    hylo_drawdown
      .outstanding()?
      .checked_add(&exo_drawdown.outstanding()?)
      .ok_or(PoolDrawdownOverflow)?,
  )
}

fn lst_position(
  header: &LstHeader,
  vault: &TokenAccount,
  stake_pool: &SplStakePool,
) -> Result<LstPosition> {
  let price_sol: LstSolPrice = header.price_sol.into();
  let prev_price_sol: LstSolPrice = header.prev_price_sol.into();
  let epoch_growth = lst_epoch_growth(&price_sol, &prev_price_sol)?;
  let lst_sol_price: UFix64<N9> = stake_pool.true_price()?.price.try_into()?;
  let sol_value = UFix64::<N9>::new(vault.amount)
    .mul_div_floor(lst_sol_price, UFix64::one())
    .ok_or(LstVaultValueOverflow)?;
  Ok(LstPosition {
    sol_value,
    epoch_growth,
  })
}

/// Builds [`StatsInputs`] from fetched accounts (order of
/// [`stats_account_keys`]).
///
/// # Errors
/// * Missing account or deserialization failure
/// * Oracle validation failure
pub fn build_stats_inputs(
  accounts: &[Account; STATS_ACCOUNT_COUNT],
  epochs_per_year: f64,
) -> Result<StatsInputs> {
  let [hylo, jitosol_header, hylosol_header, jitosol_vault, hylosol_vault, jitosol_pool_state, hylosol_pool_state, hyusd_pool, shyusd_mint, exo_pair, exo_vault, xbtc_mint, btc_usd, sol_usd, clock] =
    accounts;

  let hylo = Hylo::try_deserialize(&mut hylo.data.as_slice())?;
  let jitosol_header =
    LstHeader::try_deserialize(&mut jitosol_header.data.as_slice())?;
  let hylosol_header =
    LstHeader::try_deserialize(&mut hylosol_header.data.as_slice())?;
  let jitosol_vault =
    TokenAccount::try_deserialize(&mut jitosol_vault.data.as_slice())?;
  let hylosol_vault =
    TokenAccount::try_deserialize(&mut hylosol_vault.data.as_slice())?;
  let jitosol_pool_state = SplStakePool::from_bytes(&jitosol_pool_state.data)?;
  let hylosol_pool_state = SplStakePool::from_bytes(&hylosol_pool_state.data)?;
  let hyusd_pool =
    TokenAccount::try_deserialize(&mut hyusd_pool.data.as_slice())?;
  let shyusd_mint = Mint::try_deserialize(&mut shyusd_mint.data.as_slice())?;
  let exo_pair = ExoPair::try_deserialize(&mut exo_pair.data.as_slice())?;
  let exo_vault =
    TokenAccount::try_deserialize(&mut exo_vault.data.as_slice())?;
  let xbtc_mint = Mint::try_deserialize(&mut xbtc_mint.data.as_slice())?;
  let btc_usd = PriceUpdateV2::try_deserialize(&mut btc_usd.data.as_slice())?;
  let sol_usd = PriceUpdateV2::try_deserialize(&mut sol_usd.data.as_slice())?;
  let clock: Clock =
    bincode::deserialize(&clock.data).map_err(ClockDeserialize)?;

  let oracle_config = OracleConfig::new(
    hylo.oracle_interval_secs,
    hylo.oracle_conf_tolerance.try_into()?,
  );
  let sol_usd_spot = query_pyth_oracle(&clock, &sol_usd, oracle_config)?.spot;

  let levercoin_market_cap = exo_levercoin_market_cap(
    &clock, &exo_pair, &exo_vault, &xbtc_mint, &btc_usd,
  )?;
  let outstanding_drawdown = total_outstanding_drawdown(&hylo, &exo_pair)?;

  Ok(StatsInputs {
    current_epoch: clock.epoch,
    pool_balance: UFix64::new(hyusd_pool.amount),
    shyusd_supply: UFix64::new(shyusd_mint.supply),
    lst_harvest_cache: hylo.yield_harvest_cache.into(),
    harvest_config: hylo.yield_harvest_config.into(),
    lst_positions: vec![
      lst_position(&jitosol_header, &jitosol_vault, &jitosol_pool_state)?,
      lst_position(&hylosol_header, &hylosol_vault, &hylosol_pool_state)?,
    ],
    exo_streams: vec![ExoStream {
      collateral_mint: CBBTC::MINT,
      harvest_cache: exo_pair.borrow_rate_harvest_cache.into(),
      borrow_rate_config: exo_pair.borrow_rate_config.into(),
      levercoin_market_cap,
    }],
    sol_usd_spot,
    outstanding_drawdown,
    epochs_per_year,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  fn cache(epoch: u64, to_pool_bits: u64) -> HarvestCache {
    HarvestCache {
      epoch,
      stability_pool_cap: UFix64::<N6>::zero().into(),
      stablecoin_to_pool: UFix64::<N6>::new(to_pool_bits).into(),
    }
  }

  fn exo_stream(
    harvest_cache: HarvestCache,
    levercoin_market_cap: UFix64<N9>,
  ) -> ExoStream {
    ExoStream {
      collateral_mint: Pubkey::new_unique(),
      harvest_cache,
      borrow_rate_config: BorrowRateConfig::new(
        UFix64::<N9>::new(384_620).into(),
        UFix64::<N4>::new(500).into(),
      ),
      levercoin_market_cap,
    }
  }

  fn inputs() -> StatsInputs {
    StatsInputs {
      current_epoch: 800,
      pool_balance: UFix64::<N6>::new(1_000_000_000_000),
      shyusd_supply: UFix64::<N6>::new(950_000_000_000),
      lst_harvest_cache: cache(800, 1_000_000_000),
      harvest_config: YieldHarvestConfig {
        allocation: UFix64::<N4>::new(10_000).into(),
        fee: UFix64::<N4>::new(1_000).into(),
      },
      lst_positions: vec![LstPosition {
        sol_value: UFix64::<N9>::new(100_000_000_000_000),
        epoch_growth: UFix64::<N9>::new(500_000),
      }],
      exo_streams: vec![exo_stream(
        cache(800, 200_000_000),
        UFix64::<N9>::new(1_000_000_000_000_000),
      )],
      sol_usd_spot: UFix64::<N9>::new(150_000_000_000),
      outstanding_drawdown: UFix64::zero(),
      epochs_per_year: 182.0,
    }
  }

  #[test]
  fn annualize_compounds() {
    // 0.1% per epoch over 182 epochs ~= 19.95% APY
    let apy = annualize(UFix64::<N9>::new(1_000_000));
    assert!((apy - 0.1995).abs() < 1e-3, "apy = {apy}");
  }

  #[test]
  fn annualize_zero_rate() {
    let apy = annualize(UFix64::zero());
    assert!(apy.abs() < f64::EPSILON);
  }

  #[test]
  fn annualize_with_matches_convention() {
    let rate = UFix64::<N9>::new(1_000_000);
    let diff = (annualize_with(rate, 182.0) - annualize(rate)).abs();
    assert!(diff < f64::EPSILON);
  }

  #[test]
  fn annualize_with_lower_basis_lowers_apy() {
    let rate = UFix64::<N9>::new(2_000_000);
    let low = annualize_with(rate, 166.0);
    let high = annualize_with(rate, 182.0);
    assert!(low < high, "low = {low}, high = {high}");
    let expected = (1.002f64).powf(166.0) - 1.0;
    assert!((low - expected).abs() < 1e-12, "low = {low}");
  }

  #[test]
  fn compute_stats_uses_given_basis() -> Result<()> {
    let mut input = inputs();
    input.epochs_per_year = 100.0;
    let stats = compute_stats(&input)?;
    assert!((stats.epochs_per_year - 100.0).abs() < f64::EPSILON);
    let expected = annualize_with(stats.last_epoch_yield_rate, 100.0);
    assert!((stats.naive_apy - expected).abs() < f64::EPSILON);
    Ok(())
  }

  #[test]
  fn compute_stats_realized_sums_matching_epochs() -> Result<()> {
    let stats = compute_stats(&inputs())?;
    // 1,000 + 200 hyUSD over 1,000,000 pool = 0.12% per epoch
    assert_eq!(stats.last_epoch_yield_rate, UFix64::<N9>::new(1_200_000));
    assert!(!stats.lst_harvest.is_stale);
    assert!(!stats.exo_streams[0].harvest.is_stale);
    Ok(())
  }

  #[test]
  fn compute_stats_ignores_older_stream_epoch() -> Result<()> {
    let mut input = inputs();
    input.exo_streams[0].harvest_cache = cache(799, 200_000_000);
    let stats = compute_stats(&input)?;
    // Only the LST stream (epoch 800) counts: 0.1% per epoch
    assert_eq!(stats.last_epoch_yield_rate, UFix64::<N9>::new(1_000_000));
    assert!(stats.exo_streams[0].harvest.is_stale);
    Ok(())
  }

  #[test]
  fn compute_stats_two_exo_streams() -> Result<()> {
    let mut input = inputs();
    input.exo_streams = vec![
      exo_stream(
        cache(800, 200_000_000),
        UFix64::<N9>::new(1_000_000_000_000_000),
      ),
      exo_stream(
        cache(799, 999_000_000),
        UFix64::<N9>::new(500_000_000_000_000),
      ),
    ];
    let stats = compute_stats(&input)?;
    // lst 1,000 + stream A 200 hyUSD over 1,000,000 pool; B is stale
    assert_eq!(stats.last_epoch_yield_rate, UFix64::<N9>::new(1_200_000));
    assert!(stats.exo_streams[1].harvest.is_stale);
    let expected_exo = stats.exo_streams[0]
      .projected_inflow
      .checked_add(&stats.exo_streams[1].projected_inflow)
      .ok_or(ProjectedExoInflowOverflow)?;
    assert_eq!(stats.projected_exo_inflow, expected_exo);
    Ok(())
  }

  #[test]
  fn compute_stats_projection_pipeline() -> Result<()> {
    let stats = compute_stats(&inputs())?;
    // LST: $6,750 (Task 3 fixture); borrow: $365.389
    assert_eq!(stats.projected_lst_inflow, UFix64::<N6>::new(6_750_000_000));
    assert_eq!(
      stats.exo_streams[0].projected_inflow,
      UFix64::<N6>::new(365_389_000)
    );
    assert_eq!(stats.projected_exo_inflow, UFix64::<N6>::new(365_389_000));
    // (6,750 + 365.389) / 1,000,000 = 0.7115389% per epoch
    assert_eq!(stats.projected_epoch_rate, UFix64::<N9>::new(7_115_389));
    assert!(stats.projected_apy > stats.naive_apy);
    Ok(())
  }

  #[test]
  fn compute_stats_drawdown_reduces_projection() -> Result<()> {
    let mut input = inputs();
    input.outstanding_drawdown = UFix64::<N6>::new(7_115_389_000);
    let stats = compute_stats(&input)?;
    assert_eq!(stats.projected_epoch_rate, UFix64::zero());
    assert!(stats.projected_apy.abs() < f64::EPSILON);
    Ok(())
  }

  #[test]
  fn compute_stats_nav() -> Result<()> {
    let stats = compute_stats(&inputs())?;
    // 1,000,000 / 950,000 (ceil) = 1.052632
    assert_eq!(stats.nav, UFix64::<N6>::new(1_052_632));
    Ok(())
  }

  #[test]
  fn stats_account_keys_order() {
    assert_eq!(STATS_ACCOUNT_KEYS[0], hylo_idl::pda::HYLO);
    assert_eq!(STATS_ACCOUNT_KEYS[7], hylo_idl::pda::HYUSD_POOL);
    assert_eq!(
      STATS_ACCOUNT_KEYS[14],
      anchor_lang::solana_program::sysvar::clock::ID
    );
  }
}
