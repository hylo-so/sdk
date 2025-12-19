//! Protocol state types and deserialization
//!
//! Contains the `ProtocolState` struct and its construction from protocol
//! accounts.

use anchor_client::solana_sdk::clock::{Clock, UnixTimestamp};
use anchor_lang::AccountDeserialize;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Result};
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fee_controller::{LevercoinFees, StablecoinFees};
use hylo_core::idl::exchange::accounts::{Hylo, LstHeader};
use hylo_core::idl::stability_pool::accounts::PoolConfig;
use hylo_core::idl_type_bridge::convert_ufixvalue64;
use hylo_core::pyth::OracleConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_mode::StabilityController;
use hylo_core::total_sol_cache::TotalSolCache;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::protocol_state::ProtocolAccounts;

/// Complete snapshot of Hylo protocol state
pub struct ProtocolState<C: SolanaClock> {
  /// Exchange context with all protocol parameters
  pub exchange_context: ExchangeContext<C>,

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

    let fetched_at = clock.unix_timestamp;

    let total_sol_cache: TotalSolCache = hylo.total_sol_cache.into();

    let oracle_config = OracleConfig::new(
      hylo.oracle_interval_secs,
      convert_ufixvalue64(hylo.oracle_conf_tolerance)
        .try_into()
        .map_err(|e: anchor_lang::error::Error| anyhow!(e))?,
    );

    let stability_controller = StabilityController::new(
      convert_ufixvalue64(hylo.stability_threshold_1)
        .try_into()
        .map_err(|e: anchor_lang::error::Error| anyhow!(e))?,
      convert_ufixvalue64(hylo.stability_threshold_2)
        .try_into()
        .map_err(|e: anchor_lang::error::Error| anyhow!(e))?,
    )
    .map_err(|e: anchor_lang::error::Error| anyhow!(e))?;

    let hyusd_fees: StablecoinFees = hylo.stablecoin_fees.into();

    let xsol_fees: LevercoinFees = hylo.levercoin_fees.into();

    let exchange_context = ExchangeContext::load(
      clock,
      &total_sol_cache,
      stability_controller,
      oracle_config,
      hyusd_fees,
      xsol_fees,
      &sol_usd,
      &hyusd_mint,
      Some(&xsol_mint),
    )
    .map_err(|e: anchor_lang::error::Error| anyhow!(e))?;

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
    })
  }
}
