use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N6, N9};
use hylo_clients::prelude::{
  ExchangeClient, SimulatePrice, StabilityPoolClient,
};
use hylo_clients::protocol_state::ProtocolState;
use hylo_clients::transaction::{
  MintArgs, RedeemArgs, StabilityPoolArgs, SwapArgs,
};
use hylo_clients::util::LST;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};

use crate::{
  ComputeUnitStrategy, LstProvider, Quote, QuoteStrategy, MAX_COMPUTE_UNITS,
};

pub struct SimulationQuoteStrategy {
  exchange_client: ExchangeClient,
  stability_pool_client: StabilityPoolClient,
}

// ============================================================================
// Implementations for LST → HYUSD (mint stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuoteStrategy<L, HYUSD, C>
  for SimulationQuoteStrategy
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(&self, amount_in: u64, user: Pubkey) -> Result<Quote> {
    let inputs = MintArgs {
      amount: UFix64::<N9>::new(amount_in),
      user,
      slippage_config: None,
    };

    let (event, compute_units, vtd) = <ExchangeClient as SimulatePrice<
      L,
      HYUSD,
    >>::simulate_transaction(
      &self.exchange_client, user, inputs
    )
    .await?;

    Ok(Quote {
      amount_in,
      amount_out: event.minted.bits,
      compute_units: compute_units.unwrap_or(MAX_COMPUTE_UNITS),
      compute_unit_strategy: ComputeUnitStrategy::Simulated,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
      instructions: vtd.instructions,
      lookup_tables: vtd.lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for HYUSD → LST (redeem stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuoteStrategy<HYUSD, L, C>
  for SimulationQuoteStrategy
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(&self, amount_in: u64, user: Pubkey) -> Result<Quote> {
    let inputs = RedeemArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: None,
    };

    let (event, compute_units, vtd) = <ExchangeClient as SimulatePrice<
      HYUSD,
      L,
    >>::simulate_transaction(
      &self.exchange_client, user, inputs
    )
    .await?;

    Ok(Quote {
      amount_in,
      amount_out: event.collateral_withdrawn.bits,
      compute_units: compute_units.unwrap_or(MAX_COMPUTE_UNITS),
      compute_unit_strategy: ComputeUnitStrategy::Simulated,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
      instructions: vtd.instructions,
      lookup_tables: vtd.lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for LST → XSOL (mint levercoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuoteStrategy<L, XSOL, C>
  for SimulationQuoteStrategy
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(&self, amount_in: u64, user: Pubkey) -> Result<Quote> {
    let inputs = MintArgs {
      amount: UFix64::<N9>::new(amount_in),
      user,
      slippage_config: None,
    };

    let (event, compute_units, vtd) = <ExchangeClient as SimulatePrice<
      L,
      XSOL,
    >>::simulate_transaction(
      &self.exchange_client, user, inputs
    )
    .await?;

    Ok(Quote {
      amount_in,
      amount_out: event.minted.bits,
      compute_units: compute_units.unwrap_or(MAX_COMPUTE_UNITS),
      compute_unit_strategy: ComputeUnitStrategy::Simulated,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
      instructions: vtd.instructions,
      lookup_tables: vtd.lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for XSOL → LST (redeem levercoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuoteStrategy<XSOL, L, C>
  for SimulationQuoteStrategy
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(&self, amount_in: u64, user: Pubkey) -> Result<Quote> {
    let inputs = RedeemArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: None,
    };

    let (event, compute_units, vtd) = <ExchangeClient as SimulatePrice<
      XSOL,
      L,
    >>::simulate_transaction(
      &self.exchange_client, user, inputs
    )
    .await?;

    Ok(Quote {
      amount_in,
      amount_out: event.collateral_withdrawn.bits,
      compute_units: compute_units.unwrap_or(MAX_COMPUTE_UNITS),
      compute_unit_strategy: ComputeUnitStrategy::Simulated,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
      instructions: vtd.instructions,
      lookup_tables: vtd.lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for HYUSD → XSOL (swap stable to lever)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<HYUSD, XSOL, C> for SimulationQuoteStrategy {
  async fn get_quote(&self, amount_in: u64, user: Pubkey) -> Result<Quote> {
    let inputs = SwapArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: None,
    };

    let (event, compute_units, vtd) = <ExchangeClient as SimulatePrice<
      HYUSD,
      XSOL,
    >>::simulate_transaction(
      &self.exchange_client, user, inputs
    )
    .await?;

    Ok(Quote {
      amount_in,
      amount_out: event.levercoin_minted.bits,
      compute_units: compute_units.unwrap_or(MAX_COMPUTE_UNITS),
      compute_unit_strategy: ComputeUnitStrategy::Simulated,
      fee_amount: event.stablecoin_fees.bits,
      fee_mint: HYUSD::MINT,
      instructions: vtd.instructions,
      lookup_tables: vtd.lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for XSOL → HYUSD (swap lever to stable)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<XSOL, HYUSD, C> for SimulationQuoteStrategy {
  async fn get_quote(&self, amount_in: u64, user: Pubkey) -> Result<Quote> {
    let inputs = SwapArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: None,
    };

    let (event, compute_units, vtd) = <ExchangeClient as SimulatePrice<
      XSOL,
      HYUSD,
    >>::simulate_transaction(
      &self.exchange_client, user, inputs
    )
    .await?;

    Ok(Quote {
      amount_in,
      amount_out: event.stablecoin_minted_user.bits,
      compute_units: compute_units.unwrap_or(MAX_COMPUTE_UNITS),
      compute_unit_strategy: ComputeUnitStrategy::Simulated,
      fee_amount: event.stablecoin_minted_fees.bits,
      fee_mint: HYUSD::MINT,
      instructions: vtd.instructions,
      lookup_tables: vtd.lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for HYUSD → SHYUSD (stability pool deposit)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<HYUSD, SHYUSD, C>
  for SimulationQuoteStrategy
{
  async fn get_quote(&self, amount_in: u64, user: Pubkey) -> Result<Quote> {
    let inputs = StabilityPoolArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
    };

    let (event, compute_units, vtd) = <StabilityPoolClient as SimulatePrice<
      HYUSD,
      SHYUSD,
    >>::simulate_transaction(
      &self.stability_pool_client,
      user,
      inputs,
    )
    .await?;

    Ok(Quote {
      amount_in,
      amount_out: event.lp_token_minted.bits,
      compute_units: compute_units.unwrap_or(MAX_COMPUTE_UNITS),
      compute_unit_strategy: ComputeUnitStrategy::Simulated,
      fee_amount: 0, // UserDepositEvent has no fees
      fee_mint: HYUSD::MINT,
      instructions: vtd.instructions,
      lookup_tables: vtd.lookup_tables,
    })
  }
}
