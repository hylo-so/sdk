use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N6};
use hylo_clients::instructions::{
  InstructionBuilder, StabilityPoolInstructionBuilder,
};
use hylo_clients::prelude::{SimulatePrice, StabilityPoolClient};
use hylo_clients::transaction::StabilityPoolArgs;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::simulation_strategy::{extract_compute_units, SimulationStrategy};
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

    let (event, compute_units) = <StabilityPoolClient as SimulatePrice<
      HYUSD,
      SHYUSD,
    >>::simulate_event_with_cus(
      &self.stability_pool_client,
      user,
      StabilityPoolArgs { amount, user },
    )
    .await?;

    let instructions = <StabilityPoolInstructionBuilder as InstructionBuilder<HYUSD, SHYUSD>>::build_instructions(
      StabilityPoolArgs { amount, user },
    )?;

    let address_lookup_tables = <StabilityPoolInstructionBuilder as InstructionBuilder<
            HYUSD,
            SHYUSD,
          >>::REQUIRED_LOOKUP_TABLES
            .to_vec();

    let (compute_units, compute_unit_strategy) =
      extract_compute_units(compute_units);

    Ok(Quote {
      amount_in,
      amount_out: event.lp_token_minted.bits,
      compute_units,
      compute_unit_strategy,
      fee_amount: 0, // UserDepositEvent has no fees
      fee_mint: HYUSD::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}
