//! Read-only fetch layer for earn pool yield statistics.

use std::sync::Arc;

use anchor_client::solana_account_decoder::UiAccountEncoding;
use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::sysvar;
use anchor_lang::{AccountDeserialize, Discriminator};
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::Result;
use fix::prelude::*;
use hylo_core::exchange_context::{ExchangeContext, ExoExchangeContext};
use hylo_core::idl::exchange;
use hylo_core::idl::exchange::accounts::{ExoPair, Hylo, LstHeader};
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::pyth::{query_pyth_oracle, OracleConfig};
use hylo_core::rebalance::pool_drawdown::PoolDrawdown;
use hylo_core::util::normalize_mint_exp;
use hylo_idl::pda;
use hylo_idl::tokens::{StakePool, TokenMint, HYLOSOL, JITOSOL, SHYUSD};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::{
  RpcAccountInfoConfig, RpcProgramAccountsConfig,
};
use solana_rpc_client_api::filter::{
  Memcmp, MemcmpEncodedBytes, RpcFilterType,
};

use crate::earn_pool_stats::compute_stats;
use crate::earn_pool_yield_math::lst_epoch_growth;
use crate::error::StatsError::{
  AccountCountMismatch, ClockDeserialize, LstVaultValueOverflow,
  MissingAccounts, NoBlockAtOrAfterSlot, NoPreviousEpoch,
  NonPositiveEpochDuration, PoolDrawdownOverflow,
};
use crate::types::{EarnPoolStats, ExoSnapshot, LstPosition, StatsInputs};

/// Seconds in a Julian year.
const SECONDS_PER_YEAR: f64 = 31_557_600.0;

/// Keys of one exo pair's accounts, discovered from its onchain
/// [`ExoPair`]. The pair carries its own collateral mint and oracle, so
/// a newly registered pair needs no constant added here.
#[derive(Clone, Copy, Debug)]
pub struct ExoPairKeys {
  pub pair: Pubkey,
  pub collateral_mint: Pubkey,
  pub oracle: Pubkey,
}

impl ExoPairKeys {
  /// Accounts fetched per exo pair, in [`ExoPairKeys::keys`] order.
  pub const COUNT: usize = 5;

  #[must_use]
  pub fn new(pair: &ExoPair) -> ExoPairKeys {
    ExoPairKeys {
      pair: pda::exo_pair(pair.collateral_mint),
      collateral_mint: pair.collateral_mint,
      oracle: pair.oracle,
    }
  }

  #[must_use]
  pub fn keys(&self) -> [Pubkey; ExoPairKeys::COUNT] {
    [
      self.pair,
      self.collateral_mint,
      pda::exo_vault(self.collateral_mint),
      pda::exo_levercoin_mint(self.collateral_mint),
      self.oracle,
    ]
  }
}

/// One exo pair's deserialized accounts.
#[derive(Clone)]
pub struct ExoPairAccounts {
  pub pair: ExoPair,
  pub collateral_mint: Mint,
  pub vault: TokenAccount,
  pub levercoin_mint: Mint,
  pub collateral_usd: PriceUpdateV2,
}

/// Deserialized onchain accounts backing one stats fetch: a fixed
/// prefix in [`StatsAccounts::FIXED_KEYS`] order, then
/// [`ExoPairKeys::COUNT`] accounts per discovered exo pair.
#[derive(Clone)]
pub struct StatsAccounts {
  pub hylo: Hylo,
  pub jitosol_header: LstHeader,
  pub hylosol_header: LstHeader,
  pub jitosol_vault: TokenAccount,
  pub hylosol_vault: TokenAccount,
  pub jitosol_pool_state: SplStakePool,
  pub hylosol_pool_state: SplStakePool,
  pub hyusd_pool: TokenAccount,
  pub shyusd_mint: Mint,
  pub sol_usd: PriceUpdateV2,
  pub clock: Clock,
  pub exo_pairs: Vec<ExoPairAccounts>,
}

impl StatsAccounts {
  /// Accounts fetched regardless of how many exo pairs exist.
  pub const FIXED_COUNT: usize = 11;

  /// Keys that do not depend on the registered exo pairs, in fetch
  /// order — the same order [`StatsAccounts::from_fetched`] reads.
  pub const FIXED_KEYS: [Pubkey; StatsAccounts::FIXED_COUNT] = [
    pda::HYLO,
    pda::lst_header(JITOSOL::MINT),
    pda::lst_header(HYLOSOL::MINT),
    pda::lst_vault(JITOSOL::MINT),
    pda::lst_vault(HYLOSOL::MINT),
    JITOSOL::POOL_STATE,
    HYLOSOL::POOL_STATE,
    pda::HYUSD_POOL,
    SHYUSD::MINT,
    pda::SOL_USD_PYTH_FEED,
    sysvar::clock::ID,
  ];

  /// Full fetch list: the fixed prefix followed by each pair's accounts.
  #[must_use]
  pub fn keys(exo_pairs: &[ExoPairKeys]) -> Vec<Pubkey> {
    StatsAccounts::FIXED_KEYS
      .into_iter()
      .chain(exo_pairs.iter().flat_map(ExoPairKeys::keys))
      .collect()
  }

  /// Number of accounts fetched for `exo_pairs` registered pairs.
  #[must_use]
  pub fn count(exo_pairs: usize) -> usize {
    StatsAccounts::FIXED_COUNT + exo_pairs * ExoPairKeys::COUNT
  }

  /// Deserializes a fetched account list, erroring with the keys of
  /// any missing accounts.
  ///
  /// # Errors
  /// * Missing account, count mismatch, or deserialization failure
  pub fn from_fetched(
    fetched: Vec<Option<Account>>,
    exo_pairs: &[ExoPairKeys],
  ) -> Result<StatsAccounts> {
    StatsAccounts::validate(&fetched, exo_pairs)?;
    let accounts = fetched.into_iter().flatten().collect::<Vec<Account>>();
    let exo_pairs = accounts[StatsAccounts::FIXED_COUNT..]
      .chunks_exact(ExoPairKeys::COUNT)
      .map(|chunk| {
        Ok(ExoPairAccounts {
          pair: ExoPair::try_deserialize(&mut chunk[0].data.as_slice())?,
          collateral_mint: Mint::try_deserialize(
            &mut chunk[1].data.as_slice(),
          )?,
          vault: TokenAccount::try_deserialize(&mut chunk[2].data.as_slice())?,
          levercoin_mint: Mint::try_deserialize(&mut chunk[3].data.as_slice())?,
          collateral_usd: PriceUpdateV2::try_deserialize(
            &mut chunk[4].data.as_slice(),
          )?,
        })
      })
      .collect::<Result<Vec<ExoPairAccounts>>>()?;
    Ok(StatsAccounts {
      hylo: Hylo::try_deserialize(&mut accounts[0].data.as_slice())?,
      jitosol_header: LstHeader::try_deserialize(
        &mut accounts[1].data.as_slice(),
      )?,
      hylosol_header: LstHeader::try_deserialize(
        &mut accounts[2].data.as_slice(),
      )?,
      jitosol_vault: TokenAccount::try_deserialize(
        &mut accounts[3].data.as_slice(),
      )?,
      hylosol_vault: TokenAccount::try_deserialize(
        &mut accounts[4].data.as_slice(),
      )?,
      jitosol_pool_state: SplStakePool::from_bytes(&accounts[5].data)?,
      hylosol_pool_state: SplStakePool::from_bytes(&accounts[6].data)?,
      hyusd_pool: TokenAccount::try_deserialize(
        &mut accounts[7].data.as_slice(),
      )?,
      shyusd_mint: Mint::try_deserialize(&mut accounts[8].data.as_slice())?,
      sol_usd: PriceUpdateV2::try_deserialize(
        &mut accounts[9].data.as_slice(),
      )?,
      clock: bincode::deserialize(&accounts[10].data)
        .map_err(ClockDeserialize)?,
      exo_pairs,
    })
  }

  /// Checks the fetched list is the expected length for `exo_pairs`
  /// and has no missing accounts.
  fn validate(
    fetched: &[Option<Account>],
    exo_pairs: &[ExoPairKeys],
  ) -> Result<()> {
    let expected = StatsAccounts::count(exo_pairs.len());
    let missing = StatsAccounts::keys(exo_pairs)
      .into_iter()
      .zip(fetched)
      .filter(|(_, account)| account.is_none())
      .map(|(key, _)| key)
      .collect::<Vec<Pubkey>>();
    if fetched.len() != expected {
      Err(
        AccountCountMismatch {
          expected,
          actual: fetched.len(),
        }
        .into(),
      )
    } else if missing.is_empty() {
      Ok(())
    } else {
      Err(MissingAccounts(missing).into())
    }
  }
}

/// Read-only client for earn pool yield statistics. Needs no keypair
/// or program client.
#[derive(Clone)]
pub struct StatsClient {
  rpc: Arc<RpcClient>,
}

impl StatsClient {
  #[must_use]
  pub fn new(rpc: Arc<RpcClient>) -> StatsClient {
    StatsClient { rpc }
  }

  /// Fetches [`EarnPoolStats`] from current onchain state: one
  /// slot-consistent `get_multiple_accounts` call, plus an
  /// epoch-schedule fetch and two epoch-boundary block-time lookups to
  /// measure the last completed epoch's duration.
  ///
  /// # Errors
  /// * RPC fetch, deserialization, or oracle validation failure
  /// * Epoch duration measurement failure
  /// * Arithmetic overflow in yield math
  pub async fn earn_pool_stats(&self) -> Result<EarnPoolStats> {
    let exo_pairs = self.discover_exo_pairs().await?;
    let keys = StatsAccounts::keys(&exo_pairs);
    let fetched = self.rpc.get_multiple_accounts(&keys).await?;
    let accounts = StatsAccounts::from_fetched(fetched, &exo_pairs)?;
    // A harvest recorded at epoch H pays the yield EARNED in H-1, so the
    // two APYs annualize over different epochs whenever epoch length
    // changed between them. They coincide once the harvest has run in the
    // current epoch, which is the common case.
    let current_epoch = accounts.clock.epoch;
    let earned_epoch = last_harvest_earned_epoch(&accounts);
    let epochs_per_year = self.measure_epochs_per_year(current_epoch).await?;
    let effective_epochs_per_year = if earned_epoch == current_epoch - 1 {
      epochs_per_year
    } else {
      self.measure_epoch_duration(earned_epoch).await?
    };
    compute_stats(&build_stats_inputs(
      &accounts,
      epochs_per_year,
      effective_epochs_per_year,
    )?)
  }

  /// Discovers every registered exo pair from its onchain [`ExoPair`]
  /// account, so a newly registered collateral is picked up without a
  /// code change.
  ///
  /// Only the keys are taken from this call; the pair data itself is
  /// re-read in the main `get_multiple_accounts` batch so every value
  /// feeding the stats comes from one slot-consistent snapshot.
  ///
  /// # Errors
  /// * RPC fetch or deserialization failure
  pub async fn discover_exo_pairs(&self) -> Result<Vec<ExoPairKeys>> {
    let config = RpcProgramAccountsConfig {
      filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new(
        0,
        MemcmpEncodedBytes::Bytes(ExoPair::DISCRIMINATOR.to_vec()),
      ))]),
      account_config: RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        ..RpcAccountInfoConfig::default()
      },
      ..RpcProgramAccountsConfig::default()
    };
    let found = self
      .rpc
      .get_program_accounts_with_config(&exchange::ID, config)
      .await?;
    let mut pairs = found
      .iter()
      .map(|(_, account)| {
        let pair = ExoPair::try_deserialize(&mut account.data.as_slice())?;
        Ok(ExoPairKeys::new(&pair))
      })
      .collect::<Result<Vec<ExoPairKeys>>>()?;
    // getProgramAccounts order is unspecified; keep the fetch list and
    // the resulting stats stable across runs.
    pairs.sort_by_key(|pair| pair.collateral_mint.to_bytes());
    Ok(pairs)
  }

  /// Measures the last completed epoch's exact wall-clock duration
  /// from block times at the epoch boundary slots, returning epochs
  /// per year.
  ///
  /// # Errors
  /// * RPC failure, missing boundary blocks, or non-positive duration
  pub async fn measure_epochs_per_year(
    &self,
    current_epoch: u64,
  ) -> Result<f64> {
    let prev_epoch = current_epoch.checked_sub(1).ok_or(NoPreviousEpoch)?;
    self.measure_epoch_duration(prev_epoch).await
  }

  /// Measures `epoch`'s exact wall-clock duration from block times at its
  /// own boundary slots, returning epochs per year. The epoch must be
  /// complete.
  ///
  /// # Errors
  /// * RPC failure, missing boundary blocks, or non-positive duration
  #[allow(clippy::cast_precision_loss)]
  pub async fn measure_epoch_duration(&self, epoch: u64) -> Result<f64> {
    let schedule = self.rpc.get_epoch_schedule().await?;
    let start = schedule.get_first_slot_in_epoch(epoch);
    let end = schedule.get_first_slot_in_epoch(epoch.saturating_add(1));
    let t0 = self.block_time_at_or_after(start).await?;
    let t1 = self.block_time_at_or_after(end).await?;
    let duration = t1
      .checked_sub(t0)
      .filter(|d| *d > 0)
      .ok_or(NonPositiveEpochDuration)?;
    Ok(SECONDS_PER_YEAR / duration as f64)
  }

  /// Block time of the first block at or after `slot` (epoch boundary
  /// slots can be skipped).
  async fn block_time_at_or_after(&self, slot: u64) -> Result<i64> {
    let slots = self.rpc.get_blocks_with_limit(slot, 1).await?;
    let first = slots.first().copied().ok_or(NoBlockAtOrAfterSlot(slot))?;
    Ok(self.rpc.get_block_time(first).await?)
  }
}

/// Values an exo pair's levercoin market cap for the borrow-rate
/// projection. Mirrors hylo-quotes `build_cbbtc_exchange_context`.
fn exo_levercoin_market_cap(
  clock: &Clock,
  exo_pair: &ExoPair,
  collateral_mint: &Mint,
  exo_vault: &TokenAccount,
  levercoin_mint: &Mint,
  collateral_usd: &PriceUpdateV2,
) -> Result<UFix64<N9>> {
  let oracle_config = OracleConfig::new(
    exo_pair.oracle_interval_secs,
    exo_pair.oracle_conf_tolerance.try_into()?,
  );
  let total_collateral = normalize_mint_exp(collateral_mint, exo_vault.amount)?;
  let exo_context = ExoExchangeContext::load(
    clock.clone(),
    total_collateral,
    exo_pair.stablecoin_mint_threshold.try_into()?,
    oracle_config,
    exo_pair.levercoin_fees.into(),
    collateral_usd,
    exo_pair.virtual_stablecoin.into(),
    Some(levercoin_mint),
    exo_pair.sell_curve_config.into(),
    exo_pair.buy_curve_config.into(),
    exo_pair.levercoin_market_cap_limit.try_into()?,
  )?;
  let market_cap = exo_context.levercoin_market_cap()?;
  Ok(market_cap)
}

/// The epoch whose yield the most recent harvest paid out. A harvest
/// recorded at epoch H sweeps rewards credited at the start of H, which
/// accrued during H-1.
fn last_harvest_earned_epoch(accounts: &StatsAccounts) -> u64 {
  accounts
    .exo_pairs
    .iter()
    .map(|exo| exo.pair.borrow_rate_harvest_cache.epoch)
    .fold(accounts.hylo.yield_harvest_cache.epoch, u64::max)
    .saturating_sub(1)
}

/// Sums outstanding pool drawdown across the LST pair and every exo pair.
fn total_outstanding_drawdown(
  hylo: &Hylo,
  exo_pairs: &[ExoPairAccounts],
) -> Result<UFix64<N6>> {
  let hylo_drawdown: PoolDrawdown = hylo.pool_drawdown.into();
  exo_pairs.iter().try_fold(
    hylo_drawdown.outstanding()?,
    |acc, exo| -> Result<UFix64<N6>> {
      let drawdown: PoolDrawdown = exo.pair.pool_drawdown.into();
      Ok(
        acc
          .checked_add(&drawdown.outstanding()?)
          .ok_or(PoolDrawdownOverflow)?,
      )
    },
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

/// Builds [`StatsInputs`] from deserialized accounts.
///
/// # Errors
/// * Oracle validation failure
/// * Arithmetic overflow
pub fn build_stats_inputs(
  accounts: &StatsAccounts,
  epochs_per_year: f64,
  effective_epochs_per_year: f64,
) -> Result<StatsInputs> {
  let oracle_config = OracleConfig::new(
    accounts.hylo.oracle_interval_secs,
    accounts.hylo.oracle_conf_tolerance.try_into()?,
  );
  let sol_usd_spot =
    query_pyth_oracle(&accounts.clock, &accounts.sol_usd, oracle_config)?.spot;

  let exo_snapshots = accounts
    .exo_pairs
    .iter()
    .map(|exo| {
      Ok(ExoSnapshot {
        collateral_mint: exo.pair.collateral_mint,
        harvest_cache: exo.pair.borrow_rate_harvest_cache.into(),
        borrow_rate_config: exo.pair.borrow_rate_config.into(),
        levercoin_market_cap: exo_levercoin_market_cap(
          &accounts.clock,
          &exo.pair,
          &exo.collateral_mint,
          &exo.vault,
          &exo.levercoin_mint,
          &exo.collateral_usd,
        )?,
      })
    })
    .collect::<Result<Vec<ExoSnapshot>>>()?;
  let outstanding_drawdown =
    total_outstanding_drawdown(&accounts.hylo, &accounts.exo_pairs)?;

  Ok(StatsInputs {
    current_epoch: accounts.clock.epoch,
    pool_balance: UFix64::new(accounts.hyusd_pool.amount),
    shyusd_supply: UFix64::new(accounts.shyusd_mint.supply),
    lst_harvest_cache: accounts.hylo.yield_harvest_cache.into(),
    harvest_config: accounts.hylo.yield_harvest_config.into(),
    lst_positions: vec![
      lst_position(
        &accounts.jitosol_header,
        &accounts.jitosol_vault,
        &accounts.jitosol_pool_state,
      )?,
      lst_position(
        &accounts.hylosol_header,
        &accounts.hylosol_vault,
        &accounts.hylosol_pool_state,
      )?,
    ],
    exo_snapshots,
    sol_usd_spot,
    outstanding_drawdown,
    epochs_per_year,
    effective_epochs_per_year,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn stats_account_keys_order() {
    assert_eq!(StatsAccounts::FIXED_KEYS[0], hylo_idl::pda::HYLO);
    assert_eq!(StatsAccounts::FIXED_KEYS[7], hylo_idl::pda::HYUSD_POOL);
    assert_eq!(
      StatsAccounts::FIXED_KEYS[StatsAccounts::FIXED_COUNT - 1],
      anchor_lang::solana_program::sysvar::clock::ID
    );
  }

  #[test]
  fn keys_grow_by_five_per_exo_pair() {
    let pair = ExoPairKeys {
      pair: Pubkey::new_unique(),
      collateral_mint: Pubkey::new_unique(),
      oracle: Pubkey::new_unique(),
    };
    assert_eq!(StatsAccounts::keys(&[]).len(), StatsAccounts::FIXED_COUNT);
    assert_eq!(StatsAccounts::keys(&[pair]).len(), StatsAccounts::count(1));
    assert_eq!(
      StatsAccounts::keys(&[pair, pair]).len(),
      StatsAccounts::FIXED_COUNT + 2 * ExoPairKeys::COUNT
    );
  }

  /// The pair's own oracle is fetched, so a new collateral needs no
  /// feed constant added to this crate.
  #[test]
  fn exo_pair_keys_use_the_pairs_own_oracle() {
    let oracle = Pubkey::new_unique();
    let collateral_mint = Pubkey::new_unique();
    let keys = ExoPairKeys {
      pair: hylo_idl::pda::exo_pair(collateral_mint),
      collateral_mint,
      oracle,
    }
    .keys();
    assert_eq!(keys[1], collateral_mint);
    assert_eq!(keys[2], hylo_idl::pda::exo_vault(collateral_mint));
    assert_eq!(keys[4], oracle);
  }
}
