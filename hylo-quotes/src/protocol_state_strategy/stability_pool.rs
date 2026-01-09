//! `QuoteStrategy` implementations for stability pool pairs using
//! `TokenOperation`.

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::*;
use hylo_clients::instructions::StabilityPoolInstructionBuilder as StabilityPoolIB;
use hylo_clients::protocol_state::StateProvider;
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::token_operation::TokenOperation;
use hylo_clients::transaction::StabilityPoolArgs;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{HYUSD, SHYUSD};

use crate::protocol_state_strategy::ProtocolStateStrategy;
use crate::{
  ComputeUnitStrategy, Quote, QuoteStrategy, DEFAULT_CUS_WITH_BUFFER,
};

// HYUSD -> SHYUSD (stability pool deposit)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<HYUSD, SHYUSD, C>
  for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let op = TokenOperation::<HYUSD, SHYUSD>::compute(&state, amount_in)?;
    let args = StabilityPoolArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
    };
    let instructions =
      StabilityPoolIB::build_instructions::<HYUSD, SHYUSD>(args)?;
    let address_lookup_tables =
      StabilityPoolIB::lookup_tables::<HYUSD, SHYUSD>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.amount_out,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// SHYUSD -> HYUSD (stability pool withdrawal)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<SHYUSD, HYUSD, C>
  for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let op = TokenOperation::<SHYUSD, HYUSD>::compute(&state, amount_in)?;
    let args = StabilityPoolArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
    };
    let instructions =
      StabilityPoolIB::build_instructions::<SHYUSD, HYUSD>(args)?;
    let address_lookup_tables =
      StabilityPoolIB::lookup_tables::<SHYUSD, HYUSD>().into();
    Ok(Quote {
      amount_in,
      amount_out: op.amount_out,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}
