//! `TokenOperation` implementations for exchange pairs.

use anyhow::{ensure, Result};
use fix::prelude::*;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::lst_sol_price::LstSolPrice;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_mode::StabilityMode;
use hylo_idl::tokens::{TokenMint, HYUSD, XSOL};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{
  LstSwapOperationOutput, MintOperationOutput, OperationOutput,
  RedeemOperationOutput, SwapOperationOutput, TokenOperation,
};
use crate::util::LST;

/// Mint stablecoin (HYUSD) from LST collateral.
impl<L: LST, C: SolanaClock> TokenOperation<L, HYUSD> for ProtocolState<C> {
  type FeeExp = L::Exp;

  fn compute_quote(
    &self,
    in_amount: UFix64<L::Exp>,
  ) -> Result<MintOperationOutput> {
    ensure!(
      self.exchange_context.stability_mode <= StabilityMode::Mode1,
      "Mint operations disabled in current stability mode"
    );
    let lst_header = self.lst_header::<L>()?;
    let lst_price = lst_header.price_sol.into();
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .stablecoin_mint_fee(&lst_price, in_amount)?;
    let stablecoin_nav = self.exchange_context.stablecoin_nav()?;
    let converted = self
      .exchange_context
      .token_conversion(&lst_price)?
      .lst_to_token(amount_remaining, stablecoin_nav)?;
    let out_amount = self
      .exchange_context
      .validate_stablecoin_amount(converted)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: L::MINT,
      fee_base: in_amount,
    })
  }
}

/// Redeem stablecoin (HYUSD) for LST collateral.
impl<L: LST, C: SolanaClock> TokenOperation<HYUSD, L> for ProtocolState<C> {
  type FeeExp = L::Exp;

  fn compute_quote(
    &self,
    in_amount: UFix64<<HYUSD as TokenMint>::Exp>,
  ) -> Result<RedeemOperationOutput> {
    let lst_header = self.lst_header::<L>()?;
    let lst_price = lst_header.price_sol.into();
    let stablecoin_nav = self.exchange_context.stablecoin_nav()?;
    let lst_out = self
      .exchange_context
      .token_conversion(&lst_price)?
      .token_to_lst(in_amount, stablecoin_nav)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .stablecoin_redeem_fee(&lst_price, lst_out)?;
    Ok(OperationOutput {
      in_amount,
      out_amount: amount_remaining,
      fee_amount: fees_extracted,
      fee_mint: L::MINT,
      fee_base: lst_out,
    })
  }
}

/// Mint levercoin (XSOL) from LST collateral.
impl<L: LST, C: SolanaClock> TokenOperation<L, XSOL> for ProtocolState<C> {
  type FeeExp = L::Exp;

  fn compute_quote(
    &self,
    in_amount: UFix64<L::Exp>,
  ) -> Result<MintOperationOutput> {
    ensure!(
      self.exchange_context.stability_mode != StabilityMode::Depeg,
      "Levercoin mint disabled in current stability mode"
    );
    let lst_header = self.lst_header::<L>()?;
    let lst_price = lst_header.price_sol.into();
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .levercoin_mint_fee(&lst_price, in_amount)?;
    let levercoin_mint_nav = self.exchange_context.levercoin_mint_nav()?;
    let out_amount = self
      .exchange_context
      .token_conversion(&lst_price)?
      .lst_to_token(amount_remaining, levercoin_mint_nav)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: L::MINT,
      fee_base: in_amount,
    })
  }
}

/// Redeem levercoin (XSOL) for LST collateral.
impl<L: LST, C: SolanaClock> TokenOperation<XSOL, L> for ProtocolState<C> {
  type FeeExp = L::Exp;

  fn compute_quote(
    &self,
    in_amount: UFix64<<XSOL as TokenMint>::Exp>,
  ) -> Result<RedeemOperationOutput> {
    ensure!(
      self.exchange_context.stability_mode != StabilityMode::Depeg,
      "Levercoin redemption disabled in current stability mode"
    );
    let lst_header = self.lst_header::<L>()?;
    let lst_price = lst_header.price_sol.into();
    let xsol_nav = self.exchange_context.levercoin_redeem_nav()?;
    let lst_out = self
      .exchange_context
      .token_conversion(&lst_price)?
      .token_to_lst(in_amount, xsol_nav)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .levercoin_redeem_fee(&lst_price, lst_out)?;
    Ok(OperationOutput {
      in_amount,
      out_amount: amount_remaining,
      fee_amount: fees_extracted,
      fee_mint: L::MINT,
      fee_base: lst_out,
    })
  }
}

/// Swap stablecoin (HYUSD) to levercoin (XSOL).
impl<C: SolanaClock> TokenOperation<HYUSD, XSOL> for ProtocolState<C> {
  type FeeExp = <HYUSD as TokenMint>::Exp;

  fn compute_quote(
    &self,
    in_amount: UFix64<<HYUSD as TokenMint>::Exp>,
  ) -> Result<SwapOperationOutput> {
    ensure!(
      self.exchange_context.stability_mode != StabilityMode::Depeg,
      "Swaps are disabled in current stability mode"
    );
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .stablecoin_to_levercoin_fee(in_amount)?;
    let out_amount = self
      .exchange_context
      .swap_conversion()?
      .stable_to_lever(amount_remaining)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
    })
  }
}

/// Swap levercoin (XSOL) to stablecoin (HYUSD).
impl<C: SolanaClock> TokenOperation<XSOL, HYUSD> for ProtocolState<C> {
  type FeeExp = <HYUSD as TokenMint>::Exp;

  fn compute_quote(
    &self,
    in_amount: UFix64<<XSOL as TokenMint>::Exp>,
  ) -> Result<SwapOperationOutput> {
    ensure!(
      matches!(
        self.exchange_context.stability_mode,
        StabilityMode::Normal | StabilityMode::Mode1
      ),
      "Swaps are disabled in current stability mode"
    );
    let converted = self
      .exchange_context
      .swap_conversion()?
      .lever_to_stable(in_amount)?;
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
      in_amount,
      out_amount: amount_remaining,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: hyusd_total,
    })
  }
}

/// Swap LST -> LST.
impl<L1: LST, L2: LST, C: SolanaClock> TokenOperation<L1, L2>
  for ProtocolState<C>
{
  type FeeExp = L1::Exp;

  fn compute_quote(
    &self,
    in_amount: UFix64<L1::Exp>,
  ) -> Result<LstSwapOperationOutput> {
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self.lst_swap_config.apply_fee(in_amount)?;

    let epoch = self.exchange_context.clock.epoch();
    let lst_in_header = self.lst_header::<L1>()?;
    let lst_out_header = self.lst_header::<L2>()?;

    let in_price: LstSolPrice = lst_in_header.price_sol.into();
    let out_price: LstSolPrice = lst_out_header.price_sol.into();
    let out_amount =
      in_price.convert_lst_amount(epoch, amount_remaining, &out_price)?;

    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: L1::MINT,
      fee_base: in_amount,
    })
  }
}
