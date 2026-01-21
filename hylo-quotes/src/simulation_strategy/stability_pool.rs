use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N6};
use hylo_clients::instructions::StabilityPoolInstructionBuilder as StabilityPoolIB;
use hylo_clients::prelude::StabilityPoolClient;
use hylo_clients::syntax_helpers::{InstructionBuilderExt, SimulatePriceExt};
use hylo_clients::transaction::StabilityPoolArgs;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{HYUSD, SHYUSD};

use crate::simulated_operation::SimulatedOperationExt;
use crate::simulation_strategy::{resolve_compute_units, SimulationStrategy};
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
    let args = StabilityPoolArgs { amount, user };

    let (event, cus) = self
      .stability_pool_client
      .simulate_event_with_cus::<HYUSD, SHYUSD>(user, args)
      .await?;

    let output = StabilityPoolClient::from_event::<HYUSD, SHYUSD>(&event)?;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    let args = StabilityPoolArgs { amount, user };
    let instructions =
      StabilityPoolIB::build_instructions::<HYUSD, SHYUSD>(args)?;
    let address_lookup_tables =
      StabilityPoolIB::lookup_tables::<HYUSD, SHYUSD>().into();

    Ok(Quote {
      amount_in,
      amount_out: output.out_amount.bits,
      compute_units,
      compute_unit_strategy,
      fee_amount: output.fee_amount.bits,
      fee_mint: output.fee_mint,
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
    let args = StabilityPoolArgs { amount, user };

    let (event, cus) = self
      .stability_pool_client
      .simulate_event_with_cus::<SHYUSD, HYUSD>(user, args)
      .await?;

    let output = StabilityPoolClient::from_event::<SHYUSD, HYUSD>(&event)?;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    let args = StabilityPoolArgs { amount, user };
    let instructions =
      StabilityPoolIB::build_instructions::<SHYUSD, HYUSD>(args)?;
    let address_lookup_tables =
      StabilityPoolIB::lookup_tables::<SHYUSD, HYUSD>().into();

    Ok(Quote {
      amount_in,
      amount_out: output.out_amount.bits,
      compute_units,
      compute_unit_strategy,
      fee_amount: output.fee_amount.bits,
      fee_mint: output.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}
