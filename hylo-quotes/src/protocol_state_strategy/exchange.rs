use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fix::prelude::{UFix64, N4, N6, N9};
use hylo_clients::instructions::{
  ExchangeInstructionBuilder, InstructionBuilder,
};
use hylo_clients::protocol_state::{ProtocolState, StateProvider};
use hylo_clients::transaction::{MintArgs, RedeemArgs, SwapArgs};
use hylo_clients::util::LST;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::stability_mode::StabilityMode;
use hylo_idl::tokens::{TokenMint, HYUSD, XSOL};

use crate::protocol_state_strategy::ProtocolStateStrategy;
use crate::{
  ComputeUnitStrategy, LstProvider, Quote, QuoteStrategy,
  DEFAULT_CUS_WITH_BUFFER,
};

// ============================================================================
// Implementation for LST → HYUSD (mint stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, S: StateProvider> QuoteStrategy<L, HYUSD, Clock>
  for ProtocolStateStrategy<S>
where
  ProtocolState<Clock>: crate::LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    if state.exchange_context.stability_mode > StabilityMode::Mode1 {
      return Err(anyhow!(
        "Mint operations disabled in current stability mode"
      ));
    }

    let amount_in = UFix64::<N9>::new(amount_in);
    let lst_header = state.lst_header();
    let lst_price = lst_header.price_sol.into();

    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = state
      .exchange_context
      .stablecoin_mint_fee(&lst_price, amount_in)?;

    let stablecoin_nav = state.exchange_context.stablecoin_nav()?;

    let amount_out = {
      let converted = state
        .exchange_context
        .token_conversion(&lst_price)?
        .lst_to_token(amount_remaining, stablecoin_nav)?;
      state
        .exchange_context
        .validate_stablecoin_amount(converted)?
    };

    let instructions = <ExchangeInstructionBuilder as InstructionBuilder<
      L,
      HYUSD,
    >>::build_instructions(MintArgs {
      amount: amount_in,
      user,
      slippage_config: Some(SlippageConfig::new(
        amount_out,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    let address_lookup_tables = <ExchangeInstructionBuilder as InstructionBuilder<L, HYUSD>>::REQUIRED_LOOKUP_TABLES
      .to_vec();

    Ok(Quote {
      amount_in: amount_in.bits,
      amount_out: amount_out.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for HYUSD → LST (redeem stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, S: StateProvider> QuoteStrategy<HYUSD, L, Clock>
  for ProtocolStateStrategy<S>
where
  ProtocolState<Clock>: crate::LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    let amount_in = UFix64::<N6>::new(amount_in);
    let lst_header = state.lst_header();
    let lst_price = lst_header.price_sol.into();

    let stablecoin_nav = state.exchange_context.stablecoin_nav()?;

    let lst_out = state
      .exchange_context
      .token_conversion(&lst_price)?
      .token_to_lst(amount_in, stablecoin_nav)?;

    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = state
      .exchange_context
      .stablecoin_redeem_fee(&lst_price, lst_out)?;

    let instructions = <ExchangeInstructionBuilder as InstructionBuilder<
      HYUSD,
      L,
    >>::build_instructions(RedeemArgs {
      amount: amount_in,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(amount_remaining.bits),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    let address_lookup_tables = <ExchangeInstructionBuilder as InstructionBuilder<HYUSD, L>>::REQUIRED_LOOKUP_TABLES
      .to_vec();

    Ok(Quote {
      amount_in: amount_in.bits,
      amount_out: amount_remaining.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for LST → XSOL (mint levercoin)
// ============================================================================

#[async_trait]
impl<L: LST, S: StateProvider> QuoteStrategy<L, XSOL, Clock>
  for ProtocolStateStrategy<S>
where
  ProtocolState<Clock>: crate::LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    if state.exchange_context.stability_mode == StabilityMode::Depeg {
      return Err(anyhow!("Levercoin mint disabled in current stability mode"));
    }

    let amount_in = UFix64::<N9>::new(amount_in);
    let lst_header = state.lst_header();
    let lst_price = lst_header.price_sol.into();

    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = state
      .exchange_context
      .levercoin_mint_fee(&lst_price, amount_in)?;

    let levercoin_mint_nav = state.exchange_context.levercoin_mint_nav()?;
    let xsol_out = state
      .exchange_context
      .token_conversion(&lst_price)?
      .lst_to_token(amount_remaining, levercoin_mint_nav)?;

    let instructions = <ExchangeInstructionBuilder as InstructionBuilder<
      L,
      XSOL,
    >>::build_instructions(MintArgs {
      amount: amount_in,
      user,
      slippage_config: Some(SlippageConfig::new(
        xsol_out,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    let address_lookup_tables = <ExchangeInstructionBuilder as InstructionBuilder<L, XSOL>>::REQUIRED_LOOKUP_TABLES
      .to_vec();

    Ok(Quote {
      amount_in: amount_in.bits,
      amount_out: xsol_out.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for XSOL → LST (redeem levercoin)
// ============================================================================

#[async_trait]
impl<L: LST, S: StateProvider> QuoteStrategy<XSOL, L, Clock>
  for ProtocolStateStrategy<S>
where
  ProtocolState<Clock>: crate::LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    if state.exchange_context.stability_mode == StabilityMode::Depeg {
      return Err(anyhow!(
        "Levercoin redemption disabled in current stability mode"
      ));
    }

    let amount_in = UFix64::<N6>::new(amount_in);
    let lst_header = state.lst_header();
    let lst_price = lst_header.price_sol.into();

    let xsol_nav = state.exchange_context.levercoin_redeem_nav()?;
    let lst_out = state
      .exchange_context
      .token_conversion(&lst_price)?
      .token_to_lst(amount_in, xsol_nav)?;

    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = state
      .exchange_context
      .levercoin_redeem_fee(&lst_price, lst_out)?;

    let instructions = <ExchangeInstructionBuilder as InstructionBuilder<
      XSOL,
      L,
    >>::build_instructions(RedeemArgs {
      amount: amount_in,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(amount_remaining.bits),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    let address_lookup_tables = <ExchangeInstructionBuilder as InstructionBuilder<XSOL, L>>::REQUIRED_LOOKUP_TABLES
      .to_vec();

    Ok(Quote {
      amount_in: amount_in.bits,
      amount_out: amount_remaining.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for HYUSD → XSOL (swap stable to lever)
// ============================================================================

#[async_trait]
impl<S: StateProvider> QuoteStrategy<HYUSD, XSOL, Clock>
  for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    if state.exchange_context.stability_mode == StabilityMode::Depeg {
      return Err(anyhow!("Swaps are disabled in current stability mode"));
    }

    let amount_in = UFix64::<N6>::new(amount_in);

    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = state
      .exchange_context
      .stablecoin_to_levercoin_fee(amount_in)?;

    let xsol_out = state
      .exchange_context
      .swap_conversion()?
      .stable_to_lever(amount_remaining)?;

    let instructions = <ExchangeInstructionBuilder as InstructionBuilder<
      HYUSD,
      XSOL,
    >>::build_instructions(SwapArgs {
      amount: amount_in,
      user,
      slippage_config: Some(SlippageConfig::new(
        xsol_out,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    let address_lookup_tables = <ExchangeInstructionBuilder as InstructionBuilder<HYUSD, XSOL>>::REQUIRED_LOOKUP_TABLES
      .to_vec();

    Ok(Quote {
      amount_in: amount_in.bits,
      amount_out: xsol_out.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: fees_extracted.bits,
      fee_mint: HYUSD::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Implementation for XSOL → HYUSD (swap lever to stable)
// ============================================================================

#[async_trait]
impl<S: StateProvider> QuoteStrategy<XSOL, HYUSD, Clock>
  for ProtocolStateStrategy<S>
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    if matches!(
      state.exchange_context.stability_mode,
      StabilityMode::Mode2 | StabilityMode::Depeg
    ) {
      return Err(anyhow!("Swaps are disabled in current stability mode"));
    }

    let amount_in = UFix64::<N6>::new(amount_in);

    let hyusd_total = {
      let converted = state
        .exchange_context
        .swap_conversion()?
        .lever_to_stable(amount_in)?;
      state
        .exchange_context
        .validate_stablecoin_swap_amount(converted)
    }?;

    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = state
      .exchange_context
      .levercoin_to_stablecoin_fee(hyusd_total)?;

    let instructions = <ExchangeInstructionBuilder as InstructionBuilder<
      XSOL,
      HYUSD,
    >>::build_instructions(SwapArgs {
      amount: amount_in,
      user,
      slippage_config: Some(SlippageConfig::new(
        amount_remaining,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    })?;

    let address_lookup_tables = <ExchangeInstructionBuilder as InstructionBuilder<XSOL, HYUSD>>::REQUIRED_LOOKUP_TABLES
      .to_vec();

    Ok(Quote {
      amount_in: amount_in.bits,
      amount_out: amount_remaining.bits,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: fees_extracted.bits,
      fee_mint: HYUSD::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}
