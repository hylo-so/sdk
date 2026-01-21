use anchor_lang::prelude::Pubkey;
use anyhow::{Context, Result};
use async_trait::async_trait;
use fix::prelude::{CheckedAdd, FixExt, UFix64, N6, N9};
use hylo_clients::instructions::StabilityPoolInstructionBuilder as StabilityPoolIB;
use hylo_clients::prelude::{ProgramClient, StabilityPoolClient};
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::transaction::{StabilityPoolArgs, TransactionSyntax};
use hylo_clients::util::{parse_event, simulation_config, LST};
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::exchange::events::{
  RedeemLevercoinEventV2, RedeemStablecoinEventV2,
};
use hylo_idl::stability_pool::events::{UserDepositEvent, UserWithdrawEventV1};
use hylo_idl::tokens::{HYUSD, SHYUSD};

use crate::simulated_operation::SimulatedOperationExt;
use crate::simulation_strategy::{resolve_compute_units, SimulationStrategy};
use crate::{Local, Quote, QuoteStrategy};

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
      .simulate_event_with_cus::<HYUSD, SHYUSD, UserDepositEvent>(user, args)
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
      .simulate_event_with_cus::<SHYUSD, HYUSD, UserWithdrawEventV1>(user, args)
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

// ============================================================================
// Implementation for SHYUSD → LST (liquidation redemption)
// ============================================================================

#[async_trait]
impl<L: LST + Local, C: SolanaClock> QuoteStrategy<SHYUSD, L, C>
  for SimulationStrategy
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<Quote> {
    let amount = UFix64::<N6>::new(amount_in);
    let args = StabilityPoolArgs { amount, user };

    let tx_data = self.build_transaction_data::<SHYUSD, L>(args).await?;
    let tx = self
      .stability_pool_client
      .build_simulation_transaction(&user, &tx_data)
      .await?;
    let sim_result = self
      .stability_pool_client
      .program()
      .rpc()
      .simulate_transaction_with_config(&tx, simulation_config())
      .await?;

    // Either redemption event may be absent depending on pool state
    let from_hyusd: UFix64<N9> =
      parse_event::<RedeemStablecoinEventV2>(&sim_result)
        .map_or(UFix64::zero(), |e| UFix64::new(e.collateral_withdrawn.bits));
    let from_xsol: UFix64<N9> =
      parse_event::<RedeemLevercoinEventV2>(&sim_result)
        .map_or(UFix64::zero(), |e| UFix64::new(e.collateral_withdrawn.bits));
    let amount_out = from_hyusd
      .checked_add(&from_xsol)
      .context("amount_out overflow")?;

    let fee_from_hyusd: UFix64<N9> =
      parse_event::<RedeemStablecoinEventV2>(&sim_result)
        .map_or(UFix64::zero(), |e| UFix64::new(e.fees_deposited.bits));
    let fee_from_xsol: UFix64<N9> =
      parse_event::<RedeemLevercoinEventV2>(&sim_result)
        .map_or(UFix64::zero(), |e| UFix64::new(e.fees_deposited.bits));
    let fee_amount = fee_from_hyusd
      .checked_add(&fee_from_xsol)
      .context("fee_amount overflow")?;

    let cus = sim_result.value.units_consumed;
    let (compute_units, compute_unit_strategy) = resolve_compute_units(cus);

    Ok(Quote {
      amount_in,
      amount_out: amount_out.bits,
      compute_units,
      compute_unit_strategy,
      fee_amount: fee_amount.bits,
      fee_mint: L::MINT,
      instructions: tx_data.instructions,
      address_lookup_tables: tx_data
        .lookup_tables
        .iter()
        .map(|lut| lut.key)
        .collect(),
    })
  }
}
