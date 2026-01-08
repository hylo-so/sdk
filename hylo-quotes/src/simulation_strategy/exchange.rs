use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N4, N6, N9};
use hylo_clients::instructions::ExchangeInstructionBuilder;
use hylo_clients::prelude::ExchangeClient;
use hylo_clients::protocol_state::ProtocolState;
use hylo_clients::transaction::{MintArgs, RedeemArgs, SwapArgs};
use hylo_clients::util::LST;
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, HYUSD, XSOL};

use crate::simulation_strategy::{resolve_compute_units, SimulationStrategy};
use crate::syntax_helpers::{simulate_event_with_cus, InstructionBuilderExt};
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

    let (amount_out, fee_amount, (compute_units, compute_unit_strategy)) = {
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

      (
        event.minted.bits,
        event.fees_deposited.bits,
        resolve_compute_units(cus),
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

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<L, HYUSD>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<L, HYUSD>().into();

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

    let (amount_out, fee_amount, (compute_units, compute_unit_strategy)) = {
      let (event, cus) = simulate_event_with_cus::<ExchangeClient, HYUSD, L>(
        &self.exchange_client,
        user,
        RedeemArgs {
          amount,
          user,
          slippage_config: None,
        },
      )
      .await?;

      (
        event.collateral_withdrawn.bits,
        event.fees_deposited.bits,
        resolve_compute_units(cus),
      )
    };

    let args = RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<HYUSD, L>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<HYUSD, L>().into();

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

    let (amount_out, fee_amount, (compute_units, compute_unit_strategy)) = {
      let (event, cus) = simulate_event_with_cus::<ExchangeClient, L, XSOL>(
        &self.exchange_client,
        user,
        MintArgs {
          amount,
          user,
          slippage_config: None,
        },
      )
      .await?;

      (
        event.minted.bits,
        event.fees_deposited.bits,
        resolve_compute_units(cus),
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

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<L, XSOL>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<L, XSOL>().into();

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

    let (amount_out, fee_amount, (compute_units, compute_unit_strategy)) = {
      let (event, cus) = simulate_event_with_cus::<ExchangeClient, XSOL, L>(
        &self.exchange_client,
        user,
        RedeemArgs {
          amount,
          user,
          slippage_config: None,
        },
      )
      .await?;

      (
        event.collateral_withdrawn.bits,
        event.fees_deposited.bits,
        resolve_compute_units(cus),
      )
    };

    let args = RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<XSOL, L>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<XSOL, L>().into();

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

    let (amount_out, fee_amount, (compute_units, compute_unit_strategy)) = {
      let (event, cus) =
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

      (
        event.levercoin_minted.bits,
        event.stablecoin_fees.bits,
        resolve_compute_units(cus),
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

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<HYUSD, XSOL>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<HYUSD, XSOL>().into();

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

    let (amount_out, fee_amount, (compute_units, compute_unit_strategy)) = {
      let (event, cus) =
        simulate_event_with_cus::<ExchangeClient, XSOL, HYUSD>(
          &self.exchange_client,
          user,
          SwapArgs {
            amount,
            user,
            slippage_config: None,
          },
        )
        .await?;

      (
        event.stablecoin_minted_user.bits,
        event.stablecoin_minted_fees.bits,
        resolve_compute_units(cus),
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

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<XSOL, HYUSD>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<XSOL, HYUSD>().into();

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
