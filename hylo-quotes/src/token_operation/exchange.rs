//! `TokenOperation` implementations for exchange pairs.

use anyhow::{anyhow, ensure, Context, Result};
use fix::prelude::*;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::lst_sol_price::LstSolPrice;
use hylo_core::rebalance::mode::RebalanceMode;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{
  TokenMint, CBBTC, HYLOSOL, HYUSD, JITOSOL, USDC, XBTC, XSOL,
};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{
  LstSwapOperationOutput, MintOperationOutput, OperationOutput,
  RedeemOperationOutput, SwapOperationOutput, TokenOperation,
};
use crate::{Local, LST};

/// Mint stablecoin (HYUSD) from LST collateral.
impl<L: LST + Local, C: SolanaClock> TokenOperation<L, HYUSD>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<MintOperationOutput> {
    ensure!(
      self.exchange_context.stablecoin_mint_enabled(),
      "LST stablecoin mint disabled"
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
impl<L: LST + Local, C: SolanaClock> TokenOperation<HYUSD, L>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
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
impl<L: LST + Local, C: SolanaClock> TokenOperation<L, XSOL>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<MintOperationOutput> {
    ensure!(
      self.exchange_context.levercoin_mint_enabled(),
      "Levercoin mint disabled in current rebalance mode"
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
impl<L: LST + Local, C: SolanaClock> TokenOperation<XSOL, L>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<<XSOL as TokenMint>::Exp>,
  ) -> Result<RedeemOperationOutput> {
    ensure!(
      self.exchange_context.rebalance_mode() != RebalanceMode::Depeg,
      "Levercoin redemption disabled in current rebalance mode"
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

  fn compute_output(
    &self,
    in_amount: UFix64<<HYUSD as TokenMint>::Exp>,
  ) -> Result<SwapOperationOutput> {
    ensure!(
      self.exchange_context.rebalance_mode() != RebalanceMode::Depeg,
      "Swaps are disabled in current rebalance mode"
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

  fn compute_output(
    &self,
    in_amount: UFix64<<XSOL as TokenMint>::Exp>,
  ) -> Result<SwapOperationOutput> {
    ensure!(
      self.exchange_context.rebalance_mode() >= RebalanceMode::SellZone1,
      "Swaps are disabled in current rebalance mode"
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
impl<L1: LST + Local, L2: LST + Local, C: SolanaClock> TokenOperation<L1, L2>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
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

/// Mint stablecoin (HYUSD) from USDC.
///
/// On-chain flow: normalize USDC to N9, apply fee at N9, then convert
/// to stablecoin. Fee is denominated in USDC (at N9 precision).
impl<C: SolanaClock> TokenOperation<USDC, HYUSD> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N6, N9>> {
    let usdc_state = self.usdc_exchange_state();
    let amount_n9: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or_else(|| anyhow!("USDC N6->N9 overflow"))?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = usdc_state.apply_fee(amount_n9)?;
    let out_amount = usdc_state
      .conversion()
      .deposit_to_stablecoin(amount_remaining)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: USDC::MINT,
      fee_base: amount_n9,
    })
  }
}

/// Redeem stablecoin (HYUSD) for USDC.
///
/// On-chain flow: apply fee to HYUSD input first, then convert
/// remaining HYUSD to USDC. Fee is denominated in HYUSD.
impl<C: SolanaClock> TokenOperation<HYUSD, USDC> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N6, N6>> {
    let usdc_state = self.usdc_exchange_state();
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = usdc_state.apply_fee(in_amount)?;
    let usdc_out_n9 = usdc_state
      .conversion()
      .stablecoin_to_withdrawal(amount_remaining)?;
    let out_amount: UFix64<N6> = usdc_out_n9
      .checked_convert()
      .ok_or_else(|| anyhow!("USDC N9->N6 overflow"))?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
    })
  }
}

/// Mint stablecoin (HYUSD) from cbBTC.
impl<C: SolanaClock> TokenOperation<CBBTC, HYUSD> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N8>,
  ) -> Result<OperationOutput<N8, N6, N9>> {
    let exo = self.cbbtc_exchange_context();
    ensure!(
      exo.stablecoin_mint_enabled(),
      "Exo stablecoin mint disabled"
    );
    let collateral_n9: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or_else(|| anyhow!("cbBTC N8->N9 overflow"))?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.stablecoin_mint_fee(collateral_n9)?;
    let stablecoin_nav = exo.stablecoin_nav()?;
    let converted = exo
      .exo_conversion()
      .exo_to_token(amount_remaining, stablecoin_nav)?;
    let out_amount = exo.validate_stablecoin_amount(converted)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: CBBTC::MINT,
      fee_base: collateral_n9,
    })
  }
}

/// Redeem stablecoin (HYUSD) for cbBTC.
impl<C: SolanaClock> TokenOperation<HYUSD, CBBTC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N8, N9>> {
    let exo = self.cbbtc_exchange_context();
    let stablecoin_nav = exo.stablecoin_nav()?;
    let collateral_n9 = exo
      .exo_conversion()
      .token_to_exo(in_amount, stablecoin_nav)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.stablecoin_redeem_fee(collateral_n9)?;
    let out_amount: UFix64<N8> = amount_remaining
      .checked_convert()
      .ok_or_else(|| anyhow!("cbBTC N9->N8 overflow"))?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: CBBTC::MINT,
      fee_base: collateral_n9,
    })
  }
}

/// Mint levercoin (xBTC) from cbBTC.
impl<C: SolanaClock> TokenOperation<CBBTC, XBTC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N8>,
  ) -> Result<OperationOutput<N8, N6, N9>> {
    let exo = self.cbbtc_exchange_context();
    ensure!(
      exo.levercoin_mint_enabled(),
      "Exo levercoin mint disabled in current rebalance mode"
    );
    let collateral_n9: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or_else(|| anyhow!("cbBTC N8->N9 overflow"))?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.levercoin_mint_fee(collateral_n9)?;
    let levercoin_nav = exo.levercoin_mint_nav()?;
    let out_amount = exo
      .exo_conversion()
      .exo_to_token(amount_remaining, levercoin_nav)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: CBBTC::MINT,
      fee_base: collateral_n9,
    })
  }
}

/// Redeem levercoin (xBTC) for cbBTC.
impl<C: SolanaClock> TokenOperation<XBTC, CBBTC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N8, N9>> {
    let exo = self.cbbtc_exchange_context();
    ensure!(
      exo.rebalance_mode() > RebalanceMode::Depeg,
      "Exo levercoin redemption disabled in current rebalance mode"
    );
    let levercoin_nav = exo.levercoin_redeem_nav()?;
    let collateral_n9 = exo
      .exo_conversion()
      .token_to_exo(in_amount, levercoin_nav)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.levercoin_redeem_fee(collateral_n9)?;
    let out_amount: UFix64<N8> = amount_remaining
      .checked_convert()
      .ok_or_else(|| anyhow!("cbBTC N9->N8 overflow"))?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: CBBTC::MINT,
      fee_base: collateral_n9,
    })
  }
}

/// Swap stablecoin (HYUSD) to exo levercoin (xBTC).
impl<C: SolanaClock> TokenOperation<HYUSD, XBTC> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N6, N6>> {
    let exo = self.cbbtc_exchange_context();
    ensure!(
      exo.rebalance_mode() > RebalanceMode::Depeg,
      "Exo swaps disabled in current rebalance mode"
    );
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.stablecoin_to_levercoin_fee(in_amount)?;
    let out_amount =
      exo.swap_conversion()?.stable_to_lever(amount_remaining)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
    })
  }
}

/// Swap exo levercoin (xBTC) to stablecoin (HYUSD).
impl<C: SolanaClock> TokenOperation<XBTC, HYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N6, N6>> {
    let exo = self.cbbtc_exchange_context();
    ensure!(
      exo.rebalance_mode() >= RebalanceMode::SellZone1,
      "Exo swaps disabled in current rebalance mode"
    );
    let converted = exo.swap_conversion()?.lever_to_stable(in_amount)?;
    let hyusd_total = exo.validate_stablecoin_swap_amount(converted)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.levercoin_to_stablecoin_fee(hyusd_total)?;
    Ok(OperationOutput {
      in_amount,
      out_amount: amount_remaining,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: hyusd_total,
    })
  }
}

/// Swap `JitoSOL` for USDC.
impl<C: SolanaClock> TokenOperation<JITOSOL, USDC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<OperationOutput<N9, N6, N9>> {
    let header = self.lst_header::<JITOSOL>()?;
    let true_price = self.stake_pool::<JITOSOL>()?.true_price()?;
    let adjusted = true_price.adjust_price(header.rebalance_fee.try_into()?)?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let conversion = self
      .exchange_context
      .rebalance_buy_conversion(&adjusted, usdc_price, in_amount)?;
    let usdc_out: UFix64<N9> = conversion.lst_to_usdc(in_amount)?;
    let out_amount = usdc_out.checked_convert().context("usdc N9->N6")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: JITOSOL::MINT,
      fee_base: in_amount,
    })
  }
}

/// Swap `hyloSOL` for USDC.
impl<C: SolanaClock> TokenOperation<HYLOSOL, USDC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<OperationOutput<N9, N6, N9>> {
    let header = self.lst_header::<HYLOSOL>()?;
    let true_price = self.stake_pool::<HYLOSOL>()?.true_price()?;
    let adjusted = true_price.adjust_price(header.rebalance_fee.try_into()?)?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let conversion = self
      .exchange_context
      .rebalance_buy_conversion(&adjusted, usdc_price, in_amount)?;
    let usdc_out: UFix64<N9> = conversion.lst_to_usdc(in_amount)?;
    let out_amount = usdc_out.checked_convert().context("usdc N9->N6")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: HYLOSOL::MINT,
      fee_base: in_amount,
    })
  }
}

/// Swap USDC for `JitoSOL`.
impl<C: SolanaClock> TokenOperation<USDC, JITOSOL> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N9, N6>> {
    let normalized: UFix64<N9> =
      in_amount.checked_convert().context("usdc N6->N9")?;
    let header = self.lst_header::<JITOSOL>()?;
    let true_price = self.stake_pool::<JITOSOL>()?.true_price()?;
    let adjusted = true_price.adjust_price(header.rebalance_fee.try_into()?)?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let conversion = self
      .exchange_context
      .rebalance_sell_conversion(&adjusted, usdc_price, normalized)?;
    let out_amount = conversion.usdc_to_lst(normalized)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: USDC::MINT,
      fee_base: in_amount,
    })
  }
}

/// Swap USDC for `hyloSOL`.
impl<C: SolanaClock> TokenOperation<USDC, HYLOSOL> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N9, N6>> {
    let normalized: UFix64<N9> =
      in_amount.checked_convert().context("usdc N6->N9")?;
    let header = self.lst_header::<HYLOSOL>()?;
    let true_price = self.stake_pool::<HYLOSOL>()?.true_price()?;
    let adjusted = true_price.adjust_price(header.rebalance_fee.try_into()?)?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let conversion = self
      .exchange_context
      .rebalance_sell_conversion(&adjusted, usdc_price, normalized)?;
    let out_amount = conversion.usdc_to_lst(normalized)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: USDC::MINT,
      fee_base: in_amount,
    })
  }
}

/// Swap cbBTC for USDC.
impl<C: SolanaClock> TokenOperation<CBBTC, USDC> for ProtocolState<C> {
  type FeeExp = N8;

  fn compute_output(
    &self,
    in_amount: UFix64<N8>,
  ) -> Result<OperationOutput<N8, N6, N8>> {
    let normalized: UFix64<N9> =
      in_amount.checked_convert().context("cbbtc N8->N9")?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let conversion = self
      .cbbtc_exchange_context()
      .rebalance_buy_conversion(usdc_price, normalized)?;
    let usdc_out: UFix64<N9> = conversion.collateral_to_usdc(normalized)?;
    let out_amount = usdc_out.checked_convert().context("usdc N9->N6")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: CBBTC::MINT,
      fee_base: in_amount,
    })
  }
}

/// Swap USDC for cbBTC.
impl<C: SolanaClock> TokenOperation<USDC, CBBTC> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N8, N6>> {
    let normalized: UFix64<N9> =
      in_amount.checked_convert().context("usdc N6->N9")?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let conversion = self
      .cbbtc_exchange_context()
      .rebalance_sell_conversion(usdc_price, normalized)?;
    let collateral_out: UFix64<N9> =
      conversion.usdc_to_collateral(normalized)?;
    let out_amount =
      collateral_out.checked_convert().context("cbbtc N9->N8")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: USDC::MINT,
      fee_base: in_amount,
    })
  }
}
