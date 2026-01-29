use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N4, N6, N9};
use hylo_clients::instructions::ExchangeInstructionBuilder as ExchangeIB;
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::transaction::{LstSwapArgs, MintArgs, RedeemArgs, SwapArgs};
use hylo_clients::util::LST;
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{HYUSD, XSOL};

use crate::simulated_operation::SimulatedOperationExt;
use crate::simulation_strategy::SimulationStrategy;
use crate::{ExecutableQuote, Local, QuoteStrategy};

type MintQuote = ExecutableQuote<N9, N6, N9>;
type RedeemQuote = ExecutableQuote<N6, N9, N9>;
type SwapQuote = ExecutableQuote<N6, N6, N6>;
type LstSwapQuote = ExecutableQuote<N9, N9, N9>;

// ============================================================================
// Implementations for LST → HYUSD (mint stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST + Local, C: SolanaClock> QuoteStrategy<L, HYUSD, C>
  for SimulationStrategy
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<MintQuote> {
    let amount = UFix64::<N9>::new(amount_in);
    let sim_args = MintArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (output, cu_info) = self
      .exchange_client
      .simulate_output::<L, HYUSD>(user, sim_args)
      .await?;

    let args = MintArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        output.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<L, HYUSD>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L, HYUSD>().into();

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
// Implementation for HYUSD → LST (redeem stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST + Local, C: SolanaClock> QuoteStrategy<HYUSD, L, C>
  for SimulationStrategy
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<RedeemQuote> {
    let amount = UFix64::<N6>::new(amount_in);
    let sim_args = RedeemArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (output, cu_info) = self
      .exchange_client
      .simulate_output::<HYUSD, L>(user, sim_args)
      .await?;

    let args = RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        output.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<HYUSD, L>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<HYUSD, L>().into();

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
// Implementation for LST → XSOL (mint levercoin)
// ============================================================================

#[async_trait]
impl<L: LST + Local, C: SolanaClock> QuoteStrategy<L, XSOL, C>
  for SimulationStrategy
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<MintQuote> {
    let amount = UFix64::<N9>::new(amount_in);
    let sim_args = MintArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (output, cu_info) = self
      .exchange_client
      .simulate_output::<L, XSOL>(user, sim_args)
      .await?;

    let args = MintArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        output.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<L, XSOL>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L, XSOL>().into();

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
// Implementation for XSOL → LST (redeem levercoin)
// ============================================================================

#[async_trait]
impl<L: LST + Local, C: SolanaClock> QuoteStrategy<XSOL, L, C>
  for SimulationStrategy
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<RedeemQuote> {
    let amount = UFix64::<N6>::new(amount_in);
    let sim_args = RedeemArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (output, cu_info) = self
      .exchange_client
      .simulate_output::<XSOL, L>(user, sim_args)
      .await?;

    let args = RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        output.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<XSOL, L>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<XSOL, L>().into();

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
// Implementation for HYUSD → XSOL (swap stable to lever)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<HYUSD, XSOL, C> for SimulationStrategy {
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<SwapQuote> {
    let amount = UFix64::<N6>::new(amount_in);
    let sim_args = SwapArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (output, cu_info) = self
      .exchange_client
      .simulate_output::<HYUSD, XSOL>(user, sim_args)
      .await?;

    let args = SwapArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        output.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<HYUSD, XSOL>(args)?;
    let address_lookup_tables =
      ExchangeIB::lookup_tables::<HYUSD, XSOL>().into();

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
// Implementation for XSOL → HYUSD (swap lever to stable)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuoteStrategy<XSOL, HYUSD, C> for SimulationStrategy {
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<SwapQuote> {
    let amount = UFix64::<N6>::new(amount_in);
    let sim_args = SwapArgs {
      amount,
      user,
      slippage_config: None,
    };

    let (output, cu_info) = self
      .exchange_client
      .simulate_output::<XSOL, HYUSD>(user, sim_args)
      .await?;

    let args = SwapArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        output.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<XSOL, HYUSD>(args)?;
    let address_lookup_tables =
      ExchangeIB::lookup_tables::<XSOL, HYUSD>().into();

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
// Implementation for LST → LST swap
// ============================================================================

#[async_trait]
impl<C: SolanaClock, L1: LST + Local, L2: LST + Local> QuoteStrategy<L1, L2, C>
  for SimulationStrategy
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<LstSwapQuote> {
    let amount = UFix64::<N9>::new(amount_in);
    let sim_args = LstSwapArgs {
      amount_lst_a: amount,
      lst_a_mint: L1::MINT,
      lst_b_mint: L2::MINT,
      user,
      slippage_config: None,
    };

    let (output, cu_info) = self
      .exchange_client
      .simulate_output::<L1, L2>(user, sim_args)
      .await?;

    let args = LstSwapArgs {
      amount_lst_a: amount,
      lst_a_mint: L1::MINT,
      lst_b_mint: L2::MINT,
      user,
      slippage_config: Some(SlippageConfig::new::<N9>(
        output.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions = ExchangeIB::build_instructions::<L1, L2>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L1, L2>().into();

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
