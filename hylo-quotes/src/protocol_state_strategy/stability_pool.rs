//! `QuoteStrategy` implementations for stability pool pairs using
//! `TokenOperation`.

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::*;
use hylo_clients::instructions::{
  ExchangeInstructionBuilder as ExchangeIB,
  StabilityPoolInstructionBuilder as StabilityPoolIB,
};
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::transaction::{RedeemArgs, StabilityPoolArgs};
use hylo_clients::util::user_ata_instruction;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_pool_math::{
  amount_token_to_withdraw, stablecoin_withdrawal_fee,
};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};

use crate::protocol_state::{ProtocolState, StateProvider};
use crate::protocol_state_strategy::ProtocolStateStrategy;
use crate::token_operation::{TokenOperation, TokenOperationExt};
use crate::{
  ComputeUnitStrategy, Local, Quote, QuoteStrategy, DEFAULT_CUS_WITH_BUFFER,
  LST,
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
    let op = TokenOperation::<HYUSD, SHYUSD>::compute_quote(
      &state,
      UFix64::new(amount_in),
    )?;
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
    let op = state.quote::<SHYUSD, HYUSD>(UFix64::new(amount_in))?;
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

// SHYUSD -> LST (stability pool withdrawal + redemption)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<SHYUSD, L, C> for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    _slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;
    let lp_tokens_to_burn = UFix64::<N6>::new(amount_in);

    // Compute quote
    let op = state.quote::<SHYUSD, L>(lp_tokens_to_burn)?;

    // Recompute withdrawal amounts for instruction building
    let lp_token_supply = UFix64::new(state.shyusd_mint.supply);
    let stablecoin_in_pool = UFix64::new(state.hyusd_pool.amount);
    let levercoin_in_pool = UFix64::new(state.xsol_pool.amount);
    let stablecoin_to_withdraw = amount_token_to_withdraw(
      lp_tokens_to_burn,
      lp_token_supply,
      stablecoin_in_pool,
    )?;
    let levercoin_to_withdraw = amount_token_to_withdraw(
      lp_tokens_to_burn,
      lp_token_supply,
      levercoin_in_pool,
    )?;

    // Compute stablecoin after withdrawal fee
    let withdrawal_fee = state.pool_config.withdrawal_fee.try_into()?;
    let stablecoin_nav = state.exchange_context.stablecoin_nav()?;
    let levercoin_nav = state.exchange_context.levercoin_mint_nav()?;
    let FeeExtract {
      amount_remaining: stablecoin_amount_remaining,
      ..
    } = stablecoin_withdrawal_fee(
      stablecoin_in_pool,
      stablecoin_to_withdraw,
      stablecoin_nav,
      levercoin_to_withdraw,
      levercoin_nav,
      withdrawal_fee,
    )?;

    // Build instructions
    let withdraw_args = StabilityPoolArgs {
      amount: lp_tokens_to_burn,
      user,
    };
    let mut instructions = vec![user_ata_instruction(&user, &L::MINT)];
    instructions.extend(StabilityPoolIB::build_instructions::<SHYUSD, HYUSD>(
      withdraw_args,
    )?);

    // Redeem stablecoin if any
    if stablecoin_amount_remaining > UFix64::zero() {
      instructions.push(user_ata_instruction(&user, &HYUSD::MINT));
      let redeem_args = RedeemArgs {
        amount: stablecoin_amount_remaining,
        user,
        slippage_config: None,
      };
      instructions
        .extend(ExchangeIB::build_instructions::<HYUSD, L>(redeem_args)?);
    }

    // Redeem levercoin if any
    if levercoin_to_withdraw > UFix64::zero() {
      instructions.push(user_ata_instruction(&user, &XSOL::MINT));
      let redeem_args = RedeemArgs {
        amount: levercoin_to_withdraw,
        user,
        slippage_config: None,
      };
      instructions
        .extend(ExchangeIB::build_instructions::<XSOL, L>(redeem_args)?);
    }

    // Set up lookup tables
    let mut address_lookup_tables: Vec<Pubkey> =
      StabilityPoolIB::lookup_tables::<SHYUSD, HYUSD>().to_vec();
    address_lookup_tables.extend(ExchangeIB::lookup_tables::<HYUSD, L>());
    address_lookup_tables.dedup();

    Ok(Quote {
      amount_in,
      amount_out: op.out_amount.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER * 3,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount.bits,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}
