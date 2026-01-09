use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N4, N6, N9};
use hylo_clients::instructions::ExchangeInstructionBuilder as ExchangeIB;
use hylo_clients::protocol_state::ProtocolState;
use hylo_clients::syntax_helpers::{InstructionBuilderExt, SimulatePriceExt};
use hylo_clients::transaction::{MintArgs, RedeemArgs, SwapArgs};
use hylo_clients::util::LST;
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, HYUSD, XSOL};

use crate::simulation_strategy::{resolve_compute_units, SimulationStrategy};
use crate::{LstProvider, Quote, QuoteStrategy};

// ============================================================================
// Implementations for LST → HYUSD (mint stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuoteStrategy<L, HYUSD, C> for SimulationStrategy
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N9>::new(amount_in);
    let args = MintArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (event, cus) = self
      .exchange_client
      .simulate_event_with_cus::<L, HYUSD>(user, args)
      .await?;

    let amount_out = event.minted.bits;
    let fee_amount = event.fees_deposited.bits;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    let args = MintArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<L, HYUSD>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L, HYUSD>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: L::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for HYUSD → LST (redeem stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuoteStrategy<HYUSD, L, C> for SimulationStrategy
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N6>::new(amount_in);
    let args = RedeemArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (event, cus) = self
      .exchange_client
      .simulate_event_with_cus::<HYUSD, L>(user, args)
      .await?;

    let amount_out = event.collateral_withdrawn.bits;
    let fee_amount = event.fees_deposited.bits;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    let args = RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<HYUSD, L>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<HYUSD, L>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: L::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for LST → XSOL (mint levercoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuoteStrategy<L, XSOL, C> for SimulationStrategy
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N9>::new(amount_in);
    let args = MintArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (event, cus) = self
      .exchange_client
      .simulate_event_with_cus::<L, XSOL>(user, args)
      .await?;

    let amount_out = event.minted.bits;
    let fee_amount = event.fees_deposited.bits;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    let args = MintArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<L, XSOL>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L, XSOL>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: L::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for XSOL → LST (redeem levercoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuoteStrategy<XSOL, L, C> for SimulationStrategy
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N6>::new(amount_in);
    let args = RedeemArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (event, cus) = self
      .exchange_client
      .simulate_event_with_cus::<XSOL, L>(user, args)
      .await?;

    let amount_out = event.collateral_withdrawn.bits;
    let fee_amount = event.fees_deposited.bits;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    let args = RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<XSOL, L>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<XSOL, L>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: L::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for HYUSD → XSOL (swap stable to lever)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<HYUSD, XSOL, C> for SimulationStrategy {
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N6>::new(amount_in);
    let args = SwapArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (event, cus) = self
      .exchange_client
      .simulate_event_with_cus::<HYUSD, XSOL>(user, args)
      .await?;

    let amount_out = event.levercoin_minted.bits;
    let fee_amount = event.stablecoin_fees.bits;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    let args = SwapArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<HYUSD, XSOL>(args)?;
    let address_lookup_tables =
      ExchangeIB::lookup_tables::<HYUSD, XSOL>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: HYUSD::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for XSOL → HYUSD (swap lever to stable)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<XSOL, HYUSD, C> for SimulationStrategy {
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N6>::new(amount_in);
    let args = SwapArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (event, cus) = self
      .exchange_client
      .simulate_event_with_cus::<XSOL, HYUSD>(user, args)
      .await?;

    let amount_out = event.stablecoin_minted_user.bits;
    let fee_amount = event.stablecoin_minted_fees.bits;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    let args = SwapArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<XSOL, HYUSD>(args)?;
    let address_lookup_tables =
      ExchangeIB::lookup_tables::<XSOL, HYUSD>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: HYUSD::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}
