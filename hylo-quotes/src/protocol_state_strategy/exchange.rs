use anchor_lang::prelude::Pubkey;
use anyhow::{ensure, Result};
use async_trait::async_trait;
use fix::prelude::{UFix64, N4, N6, N9};
use hylo_clients::instructions::ExchangeInstructionBuilder;
use hylo_clients::protocol_state::{ProtocolState, StateProvider};
use hylo_clients::transaction::{MintArgs, RedeemArgs, SwapArgs};
use hylo_clients::util::LST;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_mode::StabilityMode;
use hylo_idl::tokens::{TokenMint, HYUSD, XSOL};

use crate::protocol_state_strategy::ProtocolStateStrategy;
use crate::syntax_helpers::InstructionBuilderExt;
use crate::{
  ComputeUnitStrategy, LstProvider, Quote, QuoteStrategy,
  DEFAULT_CUS_WITH_BUFFER,
};

// ============================================================================
// Implementation for LST → HYUSD (mint stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, S: StateProvider<C>, C: SolanaClock> QuoteStrategy<L, HYUSD, C>
  for ProtocolStateStrategy<S>
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    ensure!(
      state.exchange_context.stability_mode <= StabilityMode::Mode1,
      "Mint operations disabled in current stability mode"
    );

    let amount = UFix64::<N9>::new(amount_in);
    let lst_header = <ProtocolState<C> as LstProvider<L>>::lst_header(&state);
    let lst_price = lst_header.price_sol.into();

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let FeeExtract {
        fees_extracted,
        amount_remaining,
      } = state
        .exchange_context
        .stablecoin_mint_fee(&lst_price, amount)?;

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

      (
        amount_out.bits,
        fees_extracted.bits,
        DEFAULT_CUS_WITH_BUFFER,
        ComputeUnitStrategy::Estimated,
      )
    };

    let args = MintArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<L, HYUSD>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<L, HYUSD>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
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
impl<L: LST, S: StateProvider<C>, C: SolanaClock> QuoteStrategy<HYUSD, L, C>
  for ProtocolStateStrategy<S>
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    let amount = UFix64::<N6>::new(amount_in);
    let lst_header = state.lst_header();
    let lst_price = lst_header.price_sol.into();

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let stablecoin_nav = state.exchange_context.stablecoin_nav()?;

      let lst_out = state
        .exchange_context
        .token_conversion(&lst_price)?
        .token_to_lst(amount, stablecoin_nav)?;

      let FeeExtract {
        fees_extracted,
        amount_remaining,
      } = state
        .exchange_context
        .stablecoin_redeem_fee(&lst_price, lst_out)?;

      (
        amount_remaining.bits,
        fees_extracted.bits,
        DEFAULT_CUS_WITH_BUFFER,
        ComputeUnitStrategy::Estimated,
      )
    };

    let args = RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<HYUSD, L>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<HYUSD, L>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
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
impl<L: LST, S: StateProvider<C>, C: SolanaClock> QuoteStrategy<L, XSOL, C>
  for ProtocolStateStrategy<S>
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    ensure!(
      state.exchange_context.stability_mode != StabilityMode::Depeg,
      "Levercoin mint disabled in current stability mode"
    );

    let amount = UFix64::<N9>::new(amount_in);
    let lst_header = state.lst_header();
    let lst_price = lst_header.price_sol.into();

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let FeeExtract {
        fees_extracted,
        amount_remaining,
      } = state
        .exchange_context
        .levercoin_mint_fee(&lst_price, amount)?;

      let levercoin_mint_nav = state.exchange_context.levercoin_mint_nav()?;

      let xsol_out = state
        .exchange_context
        .token_conversion(&lst_price)?
        .lst_to_token(amount_remaining, levercoin_mint_nav)?;

      (
        xsol_out.bits,
        fees_extracted.bits,
        DEFAULT_CUS_WITH_BUFFER,
        ComputeUnitStrategy::Estimated,
      )
    };

    let args = MintArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<L, XSOL>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<L, XSOL>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
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
impl<L: LST, S: StateProvider<C>, C: SolanaClock> QuoteStrategy<XSOL, L, C>
  for ProtocolStateStrategy<S>
where
  ProtocolState<C>: LstProvider<L>,
{
  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<Quote> {
    let state = self.state_provider.fetch_state().await?;

    ensure!(
      state.exchange_context.stability_mode != StabilityMode::Depeg,
      "Levercoin redemption disabled in current stability mode"
    );

    let amount = UFix64::<N6>::new(amount_in);
    let lst_header = state.lst_header();
    let lst_price = lst_header.price_sol.into();

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let xsol_nav = state.exchange_context.levercoin_redeem_nav()?;
      let lst_out = state
        .exchange_context
        .token_conversion(&lst_price)?
        .token_to_lst(amount, xsol_nav)?;

      let FeeExtract {
        fees_extracted,
        amount_remaining,
      } = state
        .exchange_context
        .levercoin_redeem_fee(&lst_price, lst_out)?;

      (
        amount_remaining.bits,
        fees_extracted.bits,
        DEFAULT_CUS_WITH_BUFFER,
        ComputeUnitStrategy::Estimated,
      )
    };

    let args = RedeemArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N9>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<XSOL, L>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<XSOL, L>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
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

    ensure!(
      state.exchange_context.stability_mode != StabilityMode::Depeg,
      "Swaps are disabled in current stability mode"
    );

    let amount = UFix64::<N6>::new(amount_in);

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let FeeExtract {
        fees_extracted,
        amount_remaining,
      } = state.exchange_context.stablecoin_to_levercoin_fee(amount)?;

      let xsol_out = state
        .exchange_context
        .swap_conversion()?
        .stable_to_lever(amount_remaining)?;

      (
        xsol_out.bits,
        fees_extracted.bits,
        DEFAULT_CUS_WITH_BUFFER,
        ComputeUnitStrategy::Estimated,
      )
    };

    let args = SwapArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<HYUSD, XSOL>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<HYUSD, XSOL>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
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

    ensure!(
      matches!(
        state.exchange_context.stability_mode,
        StabilityMode::Normal | StabilityMode::Mode1
      ),
      "Swaps are disabled in current stability mode"
    );

    let amount = UFix64::<N6>::new(amount_in);

    let (amount_out, fee_amount, compute_units, compute_unit_strategy) = {
      let hyusd_total = {
        let converted = state
          .exchange_context
          .swap_conversion()?
          .lever_to_stable(amount)?;
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

      (
        amount_remaining.bits,
        fees_extracted.bits,
        DEFAULT_CUS_WITH_BUFFER,
        ComputeUnitStrategy::Estimated,
      )
    };

    let args = SwapArgs {
      amount,
      user,
      slippage_config: Some(SlippageConfig::new(
        UFix64::<N6>::new(amount_out),
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };

    let instructions =
      ExchangeInstructionBuilder::build_instructions::<XSOL, HYUSD>(args)?;
    let address_lookup_tables =
      ExchangeInstructionBuilder::lookup_tables::<XSOL, HYUSD>().into();

    Ok(Quote {
      amount_in,
      amount_out,
      compute_units,
      compute_unit_strategy,
      fee_amount,
      fee_mint: HYUSD::MINT,
      instructions,
      address_lookup_tables,
    })
  }
}
