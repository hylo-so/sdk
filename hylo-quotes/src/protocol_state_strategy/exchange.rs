//! `QuoteStrategy` implementations for exchange pairs using `TokenOperation`.

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::*;
use hylo_clients::instructions::ExchangeInstructionBuilder as ExchangeIB;
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::transaction::{LstSwapArgs, MintArgs, RedeemArgs, SwapArgs};
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{HYUSD, XSOL};

use crate::protocol_state::{ProtocolState, StateProvider};
use crate::protocol_state_strategy::ProtocolStateStrategy;
use crate::token_operation::TokenOperation;
use crate::{
  ComputeUnitStrategy, Local, Quote, QuoteStrategy, DEFAULT_CUS_WITH_BUFFER,
  LST,
};

// LST -> HYUSD (mint stablecoin)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<L, HYUSD, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<L, HYUSD>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.compute_quote(UFix64::new(amount_in))?;
    let args = MintArgs {
      amount: UFix64::<N9>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<L, HYUSD>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L, HYUSD>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.out_amount.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount.bits,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// HYUSD -> LST (redeem stablecoin)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<HYUSD, L, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<HYUSD, L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.compute_quote(UFix64::new(amount_in))?;
    let args = RedeemArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<HYUSD, L>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<HYUSD, L>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.out_amount.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount.bits,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// LST -> XSOL (mint levercoin)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<L, XSOL, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<L, XSOL>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.compute_quote(UFix64::new(amount_in))?;
    let args = MintArgs {
      amount: UFix64::<N9>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<L, XSOL>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L, XSOL>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.out_amount.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount.bits,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// XSOL -> LST (redeem levercoin)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<XSOL, L, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<XSOL, L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.compute_quote(UFix64::new(amount_in))?;
    let args = RedeemArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<XSOL, L>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<XSOL, L>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.out_amount.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount.bits,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// HYUSD -> XSOL (swap stable to lever)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<HYUSD, XSOL, C>
  for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let op = TokenOperation::<HYUSD, XSOL>::compute_quote(
      &state,
      UFix64::new(amount_in),
    )?;
    let args = SwapArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<HYUSD, XSOL>(args)?;
    let address_lookup_tables =
      ExchangeIB::lookup_tables::<HYUSD, XSOL>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.out_amount.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount.bits,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// XSOL -> HYUSD (swap lever to stable)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<XSOL, HYUSD, C>
  for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let op = TokenOperation::<XSOL, HYUSD>::compute_quote(
      &state,
      UFix64::new(amount_in),
    )?;
    let args = SwapArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<XSOL, HYUSD>(args)?;
    let address_lookup_tables =
      ExchangeIB::lookup_tables::<XSOL, HYUSD>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.out_amount.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount.bits,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// LST -> LST swap
#[async_trait]
impl<L1: LST + Local, L2: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<L1, L2, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<L1, L2>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let amount = UFix64::<N9>::new(amount_in);
    let op = state.compute_quote(amount)?;
    let args = LstSwapArgs {
      amount_lst_a: amount,
      lst_a_mint: L1::MINT,
      lst_b_mint: L2::MINT,
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<L1, L2>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L1, L2>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.out_amount.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount.bits,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}
