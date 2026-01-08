use anchor_lang::prelude::Pubkey;
use anyhow::{bail, Result};
use async_trait::async_trait;
use fix::prelude::{UFix64, N6};
use hylo_clients::instructions::StabilityPoolInstructionBuilder;
use hylo_clients::transaction::StabilityPoolArgs;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::simulation_strategy::{resolve_compute_units, SimulationStrategy};
use hylo_clients::syntax_helpers::{InstructionBuilderExt, SimulatePriceExt};
use crate::{Quote, QuoteStrategy};

// ============================================================================
// Implementation for HYUSD → SHYUSD (stability pool deposit)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<HYUSD, SHYUSD, C> for SimulationStrategy {
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N6>::new(amount_in);

    let (amount_out, fee_amount, (compute_units, compute_unit_strategy)) = {
      const FEE_AMOUNT: u64 = 0; // UserDepositEvent has no fees

      let (event, cus) = self
        .stability_pool_client
        .simulate_event_with_cus::<HYUSD, SHYUSD>(
          user,
          StabilityPoolArgs { amount, user },
        )
        .await?;

      (
        event.lp_token_minted.bits,
        FEE_AMOUNT,
        resolve_compute_units(cus),
      )
    };

    let args = StabilityPoolArgs { amount, user };

    let instructions = StabilityPoolInstructionBuilder::build_instructions::<
      HYUSD,
      SHYUSD,
    >(args)?;
    let address_lookup_tables =
      StabilityPoolInstructionBuilder::lookup_tables::<HYUSD, SHYUSD>().into();

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
// Implementation for SHYUSD → HYUSD (stability pool withdrawal)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<SHYUSD, HYUSD, C> for SimulationStrategy {
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N6>::new(amount_in);

    let (amount_out, fee_amount, (compute_units, compute_unit_strategy)) = {
      let (event, cus) = self
        .stability_pool_client
        .simulate_event_with_cus::<SHYUSD, HYUSD>(
          user,
          StabilityPoolArgs { amount, user },
        )
        .await?;

      if event.levercoin_withdrawn.bits > 0 {
        bail!("SHYUSD → HYUSD not possible: levercoin present in pool");
      }

      (
        event.stablecoin_withdrawn.bits,
        event.stablecoin_fees.bits,
        resolve_compute_units(cus),
      )
    };

    let args = StabilityPoolArgs { amount, user };

    let instructions = StabilityPoolInstructionBuilder::build_instructions::<
      SHYUSD,
      HYUSD,
    >(args)?;
    let address_lookup_tables =
      StabilityPoolInstructionBuilder::lookup_tables::<SHYUSD, HYUSD>().into();

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
