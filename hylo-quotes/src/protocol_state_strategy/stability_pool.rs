use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N6};
use hylo_clients::instructions::StabilityPoolInstructionBuilder;
use hylo_clients::protocol_state::StateProvider;
use hylo_clients::transaction::StabilityPoolArgs;
use hylo_core::stability_pool_math::{lp_token_nav, lp_token_out};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD};

use crate::protocol_state_strategy::ProtocolStateStrategy;
use crate::syntax_helpers::{build_instructions, lookup_tables};
use crate::{
  ComputeUnitStrategy, Quote, QuoteStrategy, DEFAULT_CUS_WITH_BUFFER,
};

type IB = StabilityPoolInstructionBuilder;

// ============================================================================
// Implementation for HYUSD → SHYUSD (stability pool deposit)
// ============================================================================

#[async_trait]
impl<S: StateProvider> QuoteStrategy<HYUSD, SHYUSD, Clock>
  for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    let amount_in = UFix64::<N6>::new(amount_in);

    let shyusd_nav = lp_token_nav(
      state.exchange_context.stablecoin_nav()?,
      UFix64::new(state.hyusd_pool.amount),
      state.exchange_context.levercoin_mint_nav()?,
      UFix64::new(state.xsol_pool.amount),
      UFix64::new(state.shyusd_mint.supply),
    )?;

    let shyusd_out = lp_token_out(amount_in, shyusd_nav)?;

    let args = StabilityPoolArgs {
      amount: amount_in,
      user,
    };

    Ok(Quote {
      amount_in: amount_in.bits,
      amount_out: shyusd_out.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: 0,
      fee_mint: HYUSD::MINT,
      instructions: build_instructions::<IB, HYUSD, SHYUSD>(args)?,
      address_lookup_tables: lookup_tables::<IB, HYUSD, SHYUSD>().into(),
    })
  }
}
