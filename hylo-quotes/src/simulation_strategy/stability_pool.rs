use std::iter::once;

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_lang::prelude::Pubkey;
use anyhow::{Context, Result};
use async_trait::async_trait;
use fix::prelude::{CheckedAdd, UFix64, N6, N9};
use hylo_clients::instructions::StabilityPoolInstructionBuilder as StabilityPoolIB;
use hylo_clients::prelude::{ProgramClient, VersionedTransactionData};
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::transaction::{
  BuildTransactionData, RedeemArgs, StabilityPoolArgs, TransactionSyntax,
};
use hylo_clients::util::{
  simulation_config, user_ata_instruction, EXCHANGE_LOOKUP_TABLE, LST,
  LST_REGISTRY_LOOKUP_TABLE, STABILITY_POOL_LOOKUP_TABLE,
};
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::exchange::events::{
  RedeemLevercoinEventV2, RedeemStablecoinEventV2,
};
use hylo_idl::stability_pool::events::UserWithdrawEventV1;
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};

use crate::simulated_operation::{ComputeUnitInfo, SimulatedOperationExt};
use crate::simulation_strategy::SimulationStrategy;
use crate::{ExecutableQuote, Local, QuoteStrategy};

type DepositQuote = ExecutableQuote<N6, N6, N6>;
type WithdrawQuote = ExecutableQuote<N6, N6, N6>;
type WithdrawRedeemQuote = ExecutableQuote<N6, N9, N9>;

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

// ============================================================================
// Implementation for SHYUSD → LST (liquidation redemption)
// ============================================================================

#[async_trait]
impl<L: LST + Local> BuildTransactionData<SHYUSD, L> for SimulationStrategy {
  type Inputs = StabilityPoolArgs;

  async fn build(
    &self,
    StabilityPoolArgs { amount, user }: StabilityPoolArgs,
  ) -> Result<VersionedTransactionData> {
    let withdraw_data = self
      .stability_pool_client
      .build_transaction_data::<SHYUSD, HYUSD>(StabilityPoolArgs {
        amount,
        user,
      })
      .await?;
    let withdraw_tx = self
      .stability_pool_client
      .build_simulation_transaction(&user, &withdraw_data)
      .await?;
    let withdraw_sim = self
      .stability_pool_client
      .simulate_transaction_return::<UserWithdrawEventV1>(&withdraw_tx)
      .await?;

    let mut instructions: Vec<Instruction> =
      once(user_ata_instruction(&user, &L::MINT))
        .chain(withdraw_data.instructions)
        .collect();

    if withdraw_sim.stablecoin_withdrawn.bits > 0 {
      let redeem_hyusd = self
        .exchange_client
        .build_transaction_data::<HYUSD, L>(RedeemArgs {
          amount: withdraw_sim.stablecoin_withdrawn.try_into()?,
          user,
          slippage_config: None,
        })
        .await?;
      instructions.push(user_ata_instruction(&user, &HYUSD::MINT));
      instructions.extend(redeem_hyusd.instructions);
    }

    if withdraw_sim.levercoin_withdrawn.bits > 0 {
      let redeem_xsol = self
        .exchange_client
        .build_transaction_data::<XSOL, L>(RedeemArgs {
          amount: withdraw_sim.levercoin_withdrawn.try_into()?,
          user,
          slippage_config: None,
        })
        .await?;
      instructions.push(user_ata_instruction(&user, &XSOL::MINT));
      instructions.extend(redeem_xsol.instructions);
    }

    let lookup_tables = self
      .stability_pool_client
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;

    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

#[async_trait]
impl<L: LST + Local, C: SolanaClock> QuoteStrategy<SHYUSD, L, C>
  for SimulationStrategy
{
  type FeeExp = N9;

  #[allow(clippy::too_many_lines)]
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<WithdrawRedeemQuote> {
    let amount = UFix64::<N6>::new(amount_in);

    // Simulate withdrawal to get token amounts
    let withdraw_data = self
      .stability_pool_client
      .build_transaction_data::<SHYUSD, HYUSD>(StabilityPoolArgs {
        amount,
        user,
      })
      .await?;
    let withdraw_tx = self
      .stability_pool_client
      .build_simulation_transaction(&user, &withdraw_data)
      .await?;
    let withdraw_event = self
      .stability_pool_client
      .simulate_transaction_return::<UserWithdrawEventV1>(&withdraw_tx)
      .await?;

    let mut instructions: Vec<Instruction> =
      once(user_ata_instruction(&user, &L::MINT))
        .chain(withdraw_data.instructions)
        .collect();

    let mut amount_out = UFix64::<N9>::default();
    let mut fee_amount = UFix64::<N9>::default();

    // Simulate each redeem individually for return data
    if withdraw_event.stablecoin_withdrawn.bits > 0 {
      let redeem_data = self
        .exchange_client
        .build_transaction_data::<HYUSD, L>(RedeemArgs {
          amount: withdraw_event.stablecoin_withdrawn.try_into()?,
          user,
          slippage_config: None,
        })
        .await?;
      let redeem_tx = self
        .exchange_client
        .build_simulation_transaction(&user, &redeem_data)
        .await?;
      let event = self
        .exchange_client
        .simulate_transaction_return::<RedeemStablecoinEventV2>(&redeem_tx)
        .await?;
      amount_out = amount_out
        .checked_add(&event.collateral_withdrawn.try_into()?)
        .context("amount_out overflow")?;
      fee_amount = fee_amount
        .checked_add(&event.fees_deposited.try_into()?)
        .context("fee_amount overflow")?;
      instructions.push(user_ata_instruction(&user, &HYUSD::MINT));
      instructions.extend(redeem_data.instructions);
    }

    if withdraw_event.levercoin_withdrawn.bits > 0 {
      let redeem_data = self
        .exchange_client
        .build_transaction_data::<XSOL, L>(RedeemArgs {
          amount: withdraw_event.levercoin_withdrawn.try_into()?,
          user,
          slippage_config: None,
        })
        .await?;
      let redeem_tx = self
        .exchange_client
        .build_simulation_transaction(&user, &redeem_data)
        .await?;
      let event = self
        .exchange_client
        .simulate_transaction_return::<RedeemLevercoinEventV2>(&redeem_tx)
        .await?;
      amount_out = amount_out
        .checked_add(&event.collateral_withdrawn.try_into()?)
        .context("amount_out overflow")?;
      fee_amount = fee_amount
        .checked_add(&event.fees_deposited.try_into()?)
        .context("fee_amount overflow")?;
      instructions.push(user_ata_instruction(&user, &XSOL::MINT));
      instructions.extend(redeem_data.instructions);
    }

    let lookup_tables = self
      .stability_pool_client
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;

    // Simulate combined transaction for CU estimation
    let tx_data = VersionedTransactionData::new(instructions, lookup_tables);
    let combined_tx = self
      .stability_pool_client
      .build_simulation_transaction(&user, &tx_data)
      .await?;
    let sim_result = self
      .stability_pool_client
      .program()
      .rpc()
      .simulate_transaction_with_config(&combined_tx, simulation_config())
      .await?;
    let cu_info =
      ComputeUnitInfo::from_simulation(sim_result.value.units_consumed);

    Ok(ExecutableQuote {
      amount_in: amount,
      amount_out,
      compute_units: cu_info.compute_units,
      compute_unit_strategy: cu_info.strategy,
      fee_amount,
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
