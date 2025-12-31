use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fix::prelude::{UFix64, N6};
use hylo_clients::instructions::StabilityPoolInstructionBuilder;
use hylo_clients::protocol_state::StateProvider;
use hylo_clients::transaction::StabilityPoolArgs;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::stability_pool_math::{
  amount_token_to_withdraw, lp_token_nav, lp_token_out,
};
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

    let amount = UFix64::<N6>::new(amount_in);

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      const FEE_AMOUNT: u64 = 0; // UserDepositEvent has no fees

      let shyusd_nav = lp_token_nav(
        state.exchange_context.stablecoin_nav()?,
        UFix64::new(state.hyusd_pool.amount),
        state.exchange_context.levercoin_mint_nav()?,
        UFix64::new(state.xsol_pool.amount),
        UFix64::new(state.shyusd_mint.supply),
      )?;

      let shyusd_out = lp_token_out(amount, shyusd_nav)?;

      (
        shyusd_out.bits,
        FEE_AMOUNT,
        DEFAULT_CUS_WITH_BUFFER,
        ComputeUnitStrategy::Estimated,
      )
    };

    let args = StabilityPoolArgs { amount, user };

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: HYUSD::MINT,
      instructions: build_instructions::<IB, HYUSD, SHYUSD>(args)?,
      address_lookup_tables: lookup_tables::<IB, HYUSD, SHYUSD>().into(),
    })
  }
}

// ============================================================================
// Implementation for SHYUSD → HYUSD (stability pool withdrawal)
// ============================================================================

#[async_trait]
impl<S: StateProvider> QuoteStrategy<SHYUSD, HYUSD, Clock>
  for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    if state.xsol_pool.amount > 0 {
      return Err(anyhow!(
        "SHYUSD → HYUSD not possible: levercoin present in pool"
      ));
    }

    let amount = UFix64::<N6>::new(amount_in);

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let shyusd_supply = UFix64::new(state.shyusd_mint.supply);
      let hyusd_in_pool = UFix64::new(state.hyusd_pool.amount);

      let hyusd_to_withdraw =
        amount_token_to_withdraw(amount, shyusd_supply, hyusd_in_pool)?;

      let withdrawal_fee = UFix64::new(state.pool_config.withdrawal_fee.bits);
      let FeeExtract {
        fees_extracted,
        amount_remaining,
      } = FeeExtract::new(withdrawal_fee, hyusd_to_withdraw)?;

      (
        amount_remaining.bits,
        fees_extracted.bits,
        DEFAULT_CUS_WITH_BUFFER,
        ComputeUnitStrategy::Estimated,
      )
    };

    let args = StabilityPoolArgs { amount, user };

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: HYUSD::MINT,
      instructions: build_instructions::<IB, SHYUSD, HYUSD>(args)?,
      address_lookup_tables: lookup_tables::<IB, SHYUSD, HYUSD>().into(),
    })
  }
}
