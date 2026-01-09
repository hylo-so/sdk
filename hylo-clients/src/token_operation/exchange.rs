//! `TokenOperation` implementations for exchange pairs.

use anyhow::{ensure, Result};
use fix::prelude::*;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::idl::exchange::accounts::LstHeader;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_mode::StabilityMode;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, XSOL};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{OperationOutput, TokenOperation};
use crate::util::LST;

pub(crate) trait LstProvider<L: LST> {
  fn lst_header(&self) -> &LstHeader;
}

impl<C: SolanaClock> LstProvider<JITOSOL> for ProtocolState<C> {
  fn lst_header(&self) -> &LstHeader {
    &self.jitosol_header
  }
}

impl<C: SolanaClock> LstProvider<HYLOSOL> for ProtocolState<C> {
  fn lst_header(&self) -> &LstHeader {
    &self.hylosol_header
  }
}

/// Mint stablecoin (HYUSD) from LST collateral.
impl<L: LST, C: SolanaClock> TokenOperation<L, HYUSD> for ProtocolState<C>
where
  ProtocolState<C>: LstProvider<L>,
{
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput> {
    ensure!(
      self.exchange_context.stability_mode <= StabilityMode::Mode1,
      "Mint operations disabled in current stability mode"
    );
    let amount = UFix64::<N9>::new(amount_in);
    let lst_header = <ProtocolState<C> as LstProvider<L>>::lst_header(self);
    let lst_price = lst_header.price_sol.into();
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .stablecoin_mint_fee(&lst_price, amount)?;
    let stablecoin_nav = self.exchange_context.stablecoin_nav()?;
    let converted = self
      .exchange_context
      .token_conversion(&lst_price)?
      .lst_to_token(amount_remaining, stablecoin_nav)?;
    let amount_out = self
      .exchange_context
      .validate_stablecoin_amount(converted)?
      .bits;
    Ok(OperationOutput {
      amount_out,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
    })
  }
}

/// Redeem stablecoin (HYUSD) for LST collateral.
impl<L: LST, C: SolanaClock> TokenOperation<HYUSD, L> for ProtocolState<C>
where
  ProtocolState<C>: LstProvider<L>,
{
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput> {
    let amount = UFix64::<N6>::new(amount_in);
    let lst_header = self.lst_header();
    let lst_price = lst_header.price_sol.into();
    let stablecoin_nav = self.exchange_context.stablecoin_nav()?;
    let lst_out = self
      .exchange_context
      .token_conversion(&lst_price)?
      .token_to_lst(amount, stablecoin_nav)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .stablecoin_redeem_fee(&lst_price, lst_out)?;
    Ok(OperationOutput {
      amount_out: amount_remaining.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
    })
  }
}

/// Mint levercoin (XSOL) from LST collateral.
impl<L: LST, C: SolanaClock> TokenOperation<L, XSOL> for ProtocolState<C>
where
  ProtocolState<C>: LstProvider<L>,
{
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput> {
    ensure!(
      self.exchange_context.stability_mode != StabilityMode::Depeg,
      "Levercoin mint disabled in current stability mode"
    );
    let amount = UFix64::<N9>::new(amount_in);
    let lst_header = self.lst_header();
    let lst_price = lst_header.price_sol.into();
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .levercoin_mint_fee(&lst_price, amount)?;
    let levercoin_mint_nav = self.exchange_context.levercoin_mint_nav()?;
    let xsol_out = self
      .exchange_context
      .token_conversion(&lst_price)?
      .lst_to_token(amount_remaining, levercoin_mint_nav)?;
    Ok(OperationOutput {
      amount_out: xsol_out.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
    })
  }
}

/// Redeem levercoin (XSOL) for LST collateral.
impl<L: LST, C: SolanaClock> TokenOperation<XSOL, L> for ProtocolState<C>
where
  ProtocolState<C>: LstProvider<L>,
{
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput> {
    ensure!(
      self.exchange_context.stability_mode != StabilityMode::Depeg,
      "Levercoin redemption disabled in current stability mode"
    );
    let amount = UFix64::<N6>::new(amount_in);
    let lst_header = self.lst_header();
    let lst_price = lst_header.price_sol.into();
    let xsol_nav = self.exchange_context.levercoin_redeem_nav()?;
    let lst_out = self
      .exchange_context
      .token_conversion(&lst_price)?
      .token_to_lst(amount, xsol_nav)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .levercoin_redeem_fee(&lst_price, lst_out)?;
    Ok(OperationOutput {
      amount_out: amount_remaining.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
    })
  }
}

/// Swap stablecoin (HYUSD) to levercoin (XSOL).
impl<C: SolanaClock> TokenOperation<HYUSD, XSOL> for ProtocolState<C> {
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput> {
    ensure!(
      self.exchange_context.stability_mode != StabilityMode::Depeg,
      "Swaps are disabled in current stability mode"
    );
    let amount = UFix64::<N6>::new(amount_in);
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self.exchange_context.stablecoin_to_levercoin_fee(amount)?;
    let xsol_out = self
      .exchange_context
      .swap_conversion()?
      .stable_to_lever(amount_remaining)?;
    Ok(OperationOutput {
      amount_out: xsol_out.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: HYUSD::MINT,
    })
  }
}

/// Swap levercoin (XSOL) to stablecoin (HYUSD).
impl<C: SolanaClock> TokenOperation<XSOL, HYUSD> for ProtocolState<C> {
  fn compute_quote(&self, amount_in: u64) -> Result<OperationOutput> {
    ensure!(
      matches!(
        self.exchange_context.stability_mode,
        StabilityMode::Normal | StabilityMode::Mode1
      ),
      "Swaps are disabled in current stability mode"
    );
    let amount = UFix64::<N6>::new(amount_in);
    let converted = self
      .exchange_context
      .swap_conversion()?
      .lever_to_stable(amount)?;
    let hyusd_total = self
      .exchange_context
      .validate_stablecoin_swap_amount(converted)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .levercoin_to_stablecoin_fee(hyusd_total)?;
    Ok(OperationOutput {
      amount_out: amount_remaining.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: HYUSD::MINT,
    })
  }
}
