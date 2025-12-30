use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N4, N6, N9};
use hylo_clients::instructions::{
  ExchangeInstructionBuilder, InstructionBuilder,
};
use hylo_clients::prelude::{ExchangeClient, SimulatePrice};
use hylo_clients::protocol_state::ProtocolState;
use hylo_clients::transaction::{MintArgs, RedeemArgs, SwapArgs};
use hylo_clients::util::LST;
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, HYUSD, XSOL};

use crate::simulation_strategy::{extract_compute_units, SimulationStrategy};
use crate::syntax_helpers::{
  build_instructions, lookup_tables, simulate_event_with_cus,
};
use crate::{LstProvider, Quote, QuoteStrategy};

type IB = ExchangeInstructionBuilder;

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

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let (event, cus) = simulate_event_with_cus::<ExchangeClient, L, HYUSD>(
        &self.exchange_client,
        user,
        MintArgs {
          amount,
          user,
          slippage_config: None,
        },
      )
      .await?;

      let (compute_units, compute_unit_strategy) = extract_compute_units(cus);

      (
        event.minted.bits,
        event.fees_deposited.bits,
        compute_units,
        compute_unit_strategy,
      )
    };

    let args = MintArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: L::MINT,
      instructions: build_instructions::<IB, L, HYUSD>(args)?,
      address_lookup_tables: lookup_tables::<IB, L, HYUSD>().into(),
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

    let (event, compute_units) =
      simulate_event_with_cus::<ExchangeClient, HYUSD, L>(
        &self.exchange_client,
        user,
        RedeemArgs {
          amount,
          user,
          slippage_config: None,
        },
      )
      .await?;

    let (compute_units, compute_unit_strategy) =
      extract_compute_units(compute_units);

    let instructions = build_instructions::<IB, HYUSD, L>(RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(event.collateral_withdrawn.bits),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    Ok(Quote {
      amount_in,
      amount_out: event.collateral_withdrawn.bits,
      compute_units,
      compute_unit_strategy,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
      instructions,
      address_lookup_tables: lookup_tables::<IB, HYUSD, L>().into(),
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

    let (event, compute_units) =
      simulate_event_with_cus::<ExchangeClient, L, XSOL>(
        &self.exchange_client,
        user,
        MintArgs {
          amount,
          user,
          slippage_config: None,
        },
      )
      .await?;

    let (compute_units, compute_unit_strategy) =
      extract_compute_units(compute_units);

    let instructions = build_instructions::<IB, L, XSOL>(MintArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(event.minted.bits),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    Ok(Quote {
      amount_in,
      amount_out: event.minted.bits,
      compute_units,
      compute_unit_strategy,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
      instructions,
      address_lookup_tables: lookup_tables::<IB, L, XSOL>().into(),
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

    let (event, compute_units) =
      simulate_event_with_cus::<ExchangeClient, XSOL, L>(
        &self.exchange_client,
        user,
        RedeemArgs {
          amount,
          user,
          slippage_config: None,
        },
      )
      .await?;

    let (compute_units, compute_unit_strategy) =
      extract_compute_units(compute_units);

    let instructions = build_instructions::<IB, XSOL, L>(RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(event.collateral_withdrawn.bits),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    Ok(Quote {
      amount_in,
      amount_out: event.collateral_withdrawn.bits,
      compute_units,
      compute_unit_strategy,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
      instructions,
      address_lookup_tables: lookup_tables::<IB, XSOL, L>().into(),
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

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let (event, compute_units) =
        simulate_event_with_cus::<ExchangeClient, HYUSD, XSOL>(
          &self.exchange_client,
          user,
          SwapArgs {
            amount,
            user,
            slippage_config: None,
          },
        )
        .await?;

      let (compute_units, compute_unit_strategy) =
        extract_compute_units(compute_units);

      (
        event.levercoin_minted.bits,
        event.stablecoin_fees.bits,
        compute_units,
        compute_unit_strategy,
      )
    };

    let args = SwapArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: HYUSD::MINT,
      instructions: build_instructions::<IB, HYUSD, XSOL>(args)?,
      address_lookup_tables: lookup_tables::<IB, HYUSD, XSOL>().into(),
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

    let (event, compute_units) =
      <ExchangeClient as SimulatePrice<XSOL, HYUSD>>::simulate_event_with_cus(
        &self.exchange_client,
        user,
        SwapArgs {
          amount,
          user,
          slippage_config: None,
        },
      )
      .await?;

    let instructions = <ExchangeInstructionBuilder as InstructionBuilder<
      XSOL,
      HYUSD,
    >>::build_instructions(SwapArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(event.stablecoin_minted_user.bits),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    let address_lookup_tables = <ExchangeInstructionBuilder as InstructionBuilder<XSOL, HYUSD>>::REQUIRED_LOOKUP_TABLES
        .to_vec();

    let (compute_units, compute_unit_strategy) =
      extract_compute_units(compute_units);

    Ok(Quote {
      amount_in,
      amount_out: event.stablecoin_minted_user.bits,
      compute_units,
      compute_unit_strategy,
      fee_amount: event.stablecoin_minted_fees.bits,
      fee_mint: HYUSD::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}
