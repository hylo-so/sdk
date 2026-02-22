use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N6};
use hylo_clients::instructions::StabilityPoolInstructionBuilder as StabilityPoolIB;
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::transaction::StabilityPoolArgs;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{HYUSD, SHYUSD};

use crate::simulated_operation::SimulatedOperationExt;
use crate::simulation_strategy::SimulationStrategy;
use crate::{ExecutableQuote, QuoteStrategy};

type DepositQuote = ExecutableQuote<N6, N6, N6>;
type WithdrawQuote = ExecutableQuote<N6, N6, N6>;

// ============================================================================
// Implementation for HYUSD → SHYUSD (stability pool deposit)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<HYUSD, SHYUSD, C> for SimulationStrategy {
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<DepositQuote> {
    let amount = UFix64::<N6>::new(amount_in);
    let args = StabilityPoolArgs { amount, user };

    let (output, cu_info) = self
      .stability_pool_client
      .simulate_output::<HYUSD, SHYUSD>(user, args)
      .await?;

    let args = StabilityPoolArgs { amount, user };
    let instructions =
      StabilityPoolIB::build_instructions::<HYUSD, SHYUSD>(args)?;
    let address_lookup_tables =
      StabilityPoolIB::lookup_tables::<HYUSD, SHYUSD>().into();

    Ok(ExecutableQuote {
      amount_in: output.in_amount,
      amount_out: output.out_amount,
      compute_units: cu_info.compute_units,
      compute_unit_strategy: cu_info.strategy,
      fee_amount: output.fee_amount,
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
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<WithdrawQuote> {
    let amount = UFix64::<N6>::new(amount_in);
    let args = StabilityPoolArgs { amount, user };

    let (output, cu_info) = self
      .stability_pool_client
      .simulate_output::<SHYUSD, HYUSD>(user, args)
      .await?;

    let args = StabilityPoolArgs { amount, user };
    let instructions =
      StabilityPoolIB::build_instructions::<SHYUSD, HYUSD>(args)?;
    let address_lookup_tables =
      StabilityPoolIB::lookup_tables::<SHYUSD, HYUSD>().into();

    Ok(ExecutableQuote {
      amount_in: output.in_amount,
      amount_out: output.out_amount,
      compute_units: cu_info.compute_units,
      compute_unit_strategy: cu_info.strategy,
      fee_amount: output.fee_amount,
      fee_mint: output.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}
