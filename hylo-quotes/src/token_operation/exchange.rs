//! `TokenOperation` implementations for exchange pairs.

use fix::prelude::*;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fees::controller::FeeExtract;
use hylo_core::lst::sol_price::LstSolPrice;
use hylo_core::rebalance::mode::RebalanceMode;
use hylo_core::rebalance::pnl::RebalancePnl;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::virtual_stablecoin::{validate_burn, SUPPLY_FLOOR};
use hylo_idl::tokens::{
  TokenMint, CBBTC, HYLOSOL, HYUSD, JITOSOL, USDC, XBTC, XSOL,
};

use crate::protocol_state::ProtocolState;
use crate::token_operation::{
  atom_rate, gate, linear_rate, LstSwapOperationOutput, MintOperationOutput,
  OperationOutput, RedeemOperationOutput, SwapOperationOutput, TokenOperation,
};
use crate::{Local, LST};

impl<L: LST + Local, C: SolanaClock> TokenOperation<L, HYUSD>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<MintOperationOutput, CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(self.pool_drawdown.is_repaid(), CoreError::DrawdownNotRepaid)?;
    gate(
      self.exchange_context.stablecoin_mint_enabled(),
      CoreError::OperationDisabled,
    )?;
    gate(
      self.yield_harvest_epoch == self.exchange_context.clock.epoch(),
      CoreError::YieldHarvestNotRun,
    )?;
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
    let marginal_rate = atom_rate::<N9, N6>(
      self
        .exchange_context
        .stablecoin_mint_marginal(&lst_price, in_amount)?,
    );
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: L::MINT,
      fee_base: in_amount,
      marginal_rate,
    })
  }
}

impl<L: LST + Local, C: SolanaClock> TokenOperation<HYUSD, L>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<<HYUSD as TokenMint>::Exp>,
  ) -> Result<RedeemOperationOutput, CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      self.yield_harvest_epoch == self.exchange_context.clock.epoch(),
      CoreError::YieldHarvestNotRun,
    )?;
    let lst_header = self.lst_header::<L>()?;
    let lst_price = lst_header.price_sol.into();
    let stablecoin_nav = self.exchange_context.stablecoin_nav()?;
    let lst_out = self
      .exchange_context
      .token_conversion(&lst_price)?
      .token_to_lst(in_amount, stablecoin_nav)?;
    gate(
      lst_out <= self.lst_vault_balance::<L>()?,
      CoreError::InsufficientLiquidity,
    )?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self
      .exchange_context
      .stablecoin_redeem_fee(&lst_price, lst_out)?;
    validate_burn(
      self.exchange_context.virtual_stablecoin_supply()?,
      in_amount,
      SUPPLY_FLOOR,
    )?;
    let marginal_rate = atom_rate::<N6, N9>(
      self
        .exchange_context
        .stablecoin_redeem_marginal(&lst_price, in_amount)?,
    );
    Ok(OperationOutput {
      in_amount,
      out_amount: amount_remaining,
      fee_amount: fees_extracted,
      fee_mint: L::MINT,
      fee_base: lst_out,
      marginal_rate,
    })
  }
}

impl<L: LST + Local, C: SolanaClock> TokenOperation<L, XSOL>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<MintOperationOutput, CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(self.pool_drawdown.is_repaid(), CoreError::DrawdownNotRepaid)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      self.exchange_context.levercoin_mint_enabled(),
      CoreError::OperationDisabled,
    )?;
    gate(
      self.yield_harvest_epoch == self.exchange_context.clock.epoch(),
      CoreError::YieldHarvestNotRun,
    )?;
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
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

impl<L: LST + Local, C: SolanaClock> TokenOperation<XSOL, L>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<<XSOL as TokenMint>::Exp>,
  ) -> Result<RedeemOperationOutput, CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(self.pool_drawdown.is_repaid(), CoreError::DrawdownNotRepaid)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      self.exchange_context.rebalance_mode() != RebalanceMode::Depeg,
      CoreError::OperationDisabled,
    )?;
    gate(
      self.yield_harvest_epoch == self.exchange_context.clock.epoch(),
      CoreError::YieldHarvestNotRun,
    )?;
    let lst_header = self.lst_header::<L>()?;
    let lst_price = lst_header.price_sol.into();
    let xsol_nav = self.exchange_context.levercoin_redeem_nav()?;
    let lst_out = self
      .exchange_context
      .token_conversion(&lst_price)?
      .token_to_lst(in_amount, xsol_nav)?;
    gate(
      lst_out <= self.lst_vault_balance::<L>()?,
      CoreError::InsufficientLiquidity,
    )?;
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
      marginal_rate: linear_rate(in_amount, amount_remaining)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<HYUSD, XSOL> for ProtocolState<C> {
  type FeeExp = <HYUSD as TokenMint>::Exp;

  fn compute_output(
    &self,
    in_amount: UFix64<<HYUSD as TokenMint>::Exp>,
  ) -> Result<SwapOperationOutput, CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(self.pool_drawdown.is_repaid(), CoreError::DrawdownNotRepaid)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      self.exchange_context.levercoin_mint_enabled(),
      CoreError::OperationDisabled,
    )?;
    gate(
      self.yield_harvest_epoch == self.exchange_context.clock.epoch(),
      CoreError::YieldHarvestNotRun,
    )?;
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
    validate_burn(
      self.exchange_context.virtual_stablecoin_supply()?,
      amount_remaining,
      SUPPLY_FLOOR,
    )?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<XSOL, HYUSD> for ProtocolState<C> {
  type FeeExp = <HYUSD as TokenMint>::Exp;

  fn compute_output(
    &self,
    in_amount: UFix64<<XSOL as TokenMint>::Exp>,
  ) -> Result<SwapOperationOutput, CoreError> {
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(self.pool_drawdown.is_repaid(), CoreError::DrawdownNotRepaid)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      self.exchange_context.stablecoin_mint_enabled(),
      CoreError::OperationDisabled,
    )?;
    gate(
      self.yield_harvest_epoch == self.exchange_context.clock.epoch(),
      CoreError::YieldHarvestNotRun,
    )?;
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
      marginal_rate: linear_rate(in_amount, amount_remaining)?,
    })
  }
}

impl<L1: LST + Local, L2: LST + Local, C: SolanaClock> TokenOperation<L1, L2>
  for ProtocolState<C>
{
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<LstSwapOperationOutput, CoreError> {
    let epoch = self.exchange_context.clock.epoch();
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(
      self.yield_harvest_epoch == epoch,
      CoreError::YieldHarvestNotRun,
    )?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = self.lst_swap_config.apply_fee(in_amount)?;

    let lst_in_header = self.lst_header::<L1>()?;
    let lst_out_header = self.lst_header::<L2>()?;

    let in_price: LstSolPrice = lst_in_header.price_sol.into();
    let out_price: LstSolPrice = lst_out_header.price_sol.into();
    let out_amount =
      in_price.convert_lst_amount(epoch, amount_remaining, &out_price)?;
    gate(
      out_amount <= self.lst_vault_balance::<L2>()?,
      CoreError::InsufficientLiquidity,
    )?;

    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: L1::MINT,
      fee_base: in_amount,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<USDC, HYUSD> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N6, N9>, CoreError> {
    let usdc_state = self.usdc_exchange_state();
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!usdc_state.paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    let usdc_in: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = FeeExtract::new(usdc_state.swap_fee, usdc_in)?;
    let out_amount = usdc_state
      .conversion()
      .deposit_to_stablecoin(amount_remaining)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: USDC::MINT,
      fee_base: usdc_in,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<HYUSD, USDC> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N6, N6>, CoreError> {
    let usdc_state = self.usdc_exchange_state();
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!usdc_state.paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = FeeExtract::new(usdc_state.swap_fee, in_amount)?;
    let usdc_out = usdc_state
      .conversion()
      .stablecoin_to_withdrawal(amount_remaining)?;
    let out_amount: UFix64<N6> = usdc_out
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    gate(
      out_amount <= usdc_state.vault_balance,
      CoreError::InsufficientLiquidity,
    )?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<CBBTC, HYUSD> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N8>,
  ) -> Result<OperationOutput<N8, N6, N9>, CoreError> {
    let exo = self.cbbtc_exchange_context();
    let btc_pair = &self.btc_pair_state;
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!btc_pair.paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      btc_pair.pool_drawdown.is_repaid(),
      CoreError::DrawdownNotRepaid,
    )?;
    gate(exo.stablecoin_mint_enabled(), CoreError::OperationDisabled)?;
    gate(
      btc_pair.borrow_rate_harvest_epoch == exo.clock.epoch(),
      CoreError::BorrowRateHarvestNotRun,
    )?;
    let collateral_n9: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
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
      marginal_rate: atom_rate::<N8, N6>(
        exo.stablecoin_mint_marginal(collateral_n9)?,
      ),
    })
  }
}

impl<C: SolanaClock> TokenOperation<HYUSD, CBBTC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N8, N9>, CoreError> {
    let exo = self.cbbtc_exchange_context();
    let btc_pair = &self.btc_pair_state;
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!btc_pair.paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      btc_pair.borrow_rate_harvest_epoch == exo.clock.epoch(),
      CoreError::BorrowRateHarvestNotRun,
    )?;
    let stablecoin_nav = exo.stablecoin_nav()?;
    let collateral_out = exo
      .exo_conversion()
      .token_to_exo(in_amount, stablecoin_nav)?;
    gate(
      collateral_out <= exo.total_collateral,
      CoreError::InsufficientLiquidity,
    )?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.stablecoin_redeem_fee(collateral_out)?;
    validate_burn(
      exo.virtual_stablecoin_supply()?,
      in_amount,
      btc_pair.supply_floor,
    )?;
    let out_amount: UFix64<N8> = amount_remaining
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: CBBTC::MINT,
      fee_base: collateral_out,
      marginal_rate: atom_rate::<N6, N8>(
        exo.stablecoin_redeem_marginal(in_amount)?,
      ),
    })
  }
}

impl<C: SolanaClock> TokenOperation<CBBTC, XBTC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N8>,
  ) -> Result<OperationOutput<N8, N6, N9>, CoreError> {
    let exo = self.cbbtc_exchange_context();
    let btc_pair = &self.btc_pair_state;
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!btc_pair.paused, CoreError::PairPaused)?;
    gate(
      btc_pair.pool_drawdown.is_repaid(),
      CoreError::DrawdownNotRepaid,
    )?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(exo.levercoin_mint_enabled(), CoreError::OperationDisabled)?;
    gate(
      btc_pair.borrow_rate_harvest_epoch == exo.clock.epoch(),
      CoreError::BorrowRateHarvestNotRun,
    )?;
    let collateral_in: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.levercoin_mint_fee(collateral_in)?;
    let levercoin_nav = exo.levercoin_mint_nav()?;
    let out_amount = exo
      .exo_conversion()
      .exo_to_token(amount_remaining, levercoin_nav)?;
    exo
      .levercoin_market_cap_limiter()?
      .validate_token_out(out_amount)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: CBBTC::MINT,
      fee_base: collateral_in,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<XBTC, CBBTC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N8, N9>, CoreError> {
    let exo = self.cbbtc_exchange_context();
    let btc_pair = &self.btc_pair_state;
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!btc_pair.paused, CoreError::PairPaused)?;
    gate(
      btc_pair.pool_drawdown.is_repaid(),
      CoreError::DrawdownNotRepaid,
    )?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      exo.rebalance_mode() != RebalanceMode::Depeg,
      CoreError::OperationDisabled,
    )?;
    gate(
      btc_pair.borrow_rate_harvest_epoch == exo.clock.epoch(),
      CoreError::BorrowRateHarvestNotRun,
    )?;
    let levercoin_nav = exo.levercoin_redeem_nav()?;
    let collateral_out = exo
      .exo_conversion()
      .token_to_exo(in_amount, levercoin_nav)?;
    gate(
      collateral_out <= exo.total_collateral,
      CoreError::InsufficientLiquidity,
    )?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.levercoin_redeem_fee(collateral_out)?;
    let out_amount: UFix64<N8> = amount_remaining
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: CBBTC::MINT,
      fee_base: collateral_out,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<HYUSD, XBTC> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N6, N6>, CoreError> {
    let exo = self.cbbtc_exchange_context();
    let btc_pair = &self.btc_pair_state;
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!btc_pair.paused, CoreError::PairPaused)?;
    gate(
      btc_pair.pool_drawdown.is_repaid(),
      CoreError::DrawdownNotRepaid,
    )?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(exo.levercoin_mint_enabled(), CoreError::OperationDisabled)?;
    gate(
      btc_pair.borrow_rate_harvest_epoch == exo.clock.epoch(),
      CoreError::BorrowRateHarvestNotRun,
    )?;
    let FeeExtract {
      fees_extracted,
      amount_remaining,
    } = exo.stablecoin_to_levercoin_fee(in_amount)?;
    let out_amount =
      exo.swap_conversion()?.stable_to_lever(amount_remaining)?;
    exo
      .levercoin_market_cap_limiter()?
      .validate_token_out(out_amount)?;
    validate_burn(
      exo.virtual_stablecoin_supply()?,
      amount_remaining,
      btc_pair.supply_floor,
    )?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: fees_extracted,
      fee_mint: HYUSD::MINT,
      fee_base: in_amount,
      marginal_rate: linear_rate(in_amount, out_amount)?,
    })
  }
}

impl<C: SolanaClock> TokenOperation<XBTC, HYUSD> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N6, N6>, CoreError> {
    let exo = self.cbbtc_exchange_context();
    let btc_pair = &self.btc_pair_state;
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!btc_pair.paused, CoreError::PairPaused)?;
    gate(
      btc_pair.pool_drawdown.is_repaid(),
      CoreError::DrawdownNotRepaid,
    )?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(exo.stablecoin_mint_enabled(), CoreError::OperationDisabled)?;
    gate(
      btc_pair.borrow_rate_harvest_epoch == exo.clock.epoch(),
      CoreError::BorrowRateHarvestNotRun,
    )?;
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
      marginal_rate: linear_rate(in_amount, amount_remaining)?,
    })
  }
}

impl<C: SolanaClock> ProtocolState<C> {
  /// Mirrors rebalance `PnL` settlement against the earn pool for the
  /// pair behind `context`, with that pair's virtual supply `floor`.
  fn validate_pnl_settlement(
    &self,
    context: &impl ExchangeContext,
    floor: UFix64<N6>,
    pnl: RebalancePnl,
  ) -> Result<(), CoreError> {
    match pnl {
      RebalancePnl::Profit(profit) => {
        context.validate_stablecoin_pnl_profit(profit).map(|_| ())
      }
      RebalancePnl::Loss(loss) => {
        gate(
          UFix64::new(self.hyusd_pool.amount) >= loss,
          CoreError::InsufficientEarnPoolLiquidity,
        )?;
        validate_burn(context.virtual_stablecoin_supply()?, loss, floor)
          .map(|_| ())
      }
      RebalancePnl::NoChange => Ok(()),
    }
  }

  /// Quotes a buy-side rebalance swap (LST in, USDC out).
  fn rebalance_buy_quote<L: LST + Local>(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<OperationOutput<N9, N6, N9>, CoreError> {
    let epoch = self.exchange_context.clock.epoch();
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(!self.usdc_exchange_state().paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(self.pool_drawdown.is_repaid(), CoreError::DrawdownNotRepaid)?;
    gate(
      self.yield_harvest_epoch == epoch,
      CoreError::YieldHarvestNotRun,
    )?;
    gate(
      self.exchange_context.rebalance_buy_active(),
      CoreError::OperationDisabled,
    )?;
    let header = self.lst_header::<L>()?;
    let lst_price: LstSolPrice = header.price_sol.into();
    let true_price = self.stake_pool::<L>()?.true_price()?;
    let adjusted = true_price.adjust_price(header.rebalance_fee.try_into()?)?;
    let buy_target = adjusted.convert_sol_to_lst(
      self.exchange_context.rebalance_buy_target()?,
      epoch,
    )?;
    gate(
      in_amount <= buy_target,
      CoreError::RebalanceBuyTargetExceeded,
    )?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let conversion = self
      .exchange_context
      .rebalance_buy_conversion(&adjusted, usdc_price, in_amount)?;
    let usdc_out: UFix64<N9> = conversion.lst_to_usdc(in_amount)?;
    let out_amount = usdc_out
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    gate(
      out_amount <= self.usdc_exchange_state().vault_balance,
      CoreError::InsufficientLiquidity,
    )?;
    let stablecoin_moved = self
      .usdc_exchange_state()
      .conversion()
      .withdrawal_to_stablecoin(usdc_out)?;
    let pnl = self.exchange_context.rebalance_pnl_buy_side(
      &lst_price,
      in_amount,
      stablecoin_moved,
    )?;
    self.validate_pnl_settlement(&self.exchange_context, SUPPLY_FLOOR, pnl)?;
    let marginal_rate = atom_rate::<N9, N6>(
      self
        .exchange_context
        .rebalance_buy_marginal(&adjusted, usdc_price, in_amount)?,
    );
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: L::MINT,
      fee_base: in_amount,
      marginal_rate,
    })
  }

  /// Quotes a sell-side rebalance swap (USDC in, LST out).
  ///
  /// Input cap is a conservative spot-priced bound on the onchain gates.
  fn rebalance_sell_quote<L: LST + Local>(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N9, N6>, CoreError> {
    let epoch = self.exchange_context.clock.epoch();
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!self.lst_pair_paused, CoreError::PairPaused)?;
    gate(!self.usdc_exchange_state().paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(self.pool_drawdown.is_repaid(), CoreError::DrawdownNotRepaid)?;
    gate(
      self.yield_harvest_epoch == epoch,
      CoreError::YieldHarvestNotRun,
    )?;
    gate(
      self.exchange_context.rebalance_sell_active(),
      CoreError::OperationDisabled,
    )?;
    let normalized: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    let header = self.lst_header::<L>()?;
    let lst_price: LstSolPrice = header.price_sol.into();
    let rebalance_fee = header.rebalance_fee.try_into()?;
    let true_price = self.stake_pool::<L>()?.true_price()?;
    let adjusted = true_price.adjust_price(rebalance_fee)?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let max_usdc_in = self.exchange_context.max_rebalance_sell_usdc(
      *self.stake_pool::<L>()?,
      rebalance_fee,
      self.lst_vault_balance::<L>()?,
      usdc_price,
      SUPPLY_FLOOR,
    )?;
    gate(normalized <= max_usdc_in, CoreError::InsufficientLiquidity)?;
    let conversion = self
      .exchange_context
      .rebalance_sell_conversion(&adjusted, usdc_price, normalized)?;
    let out_amount = conversion.usdc_to_lst(normalized)?;
    let stablecoin_moved = self
      .usdc_exchange_state()
      .conversion()
      .deposit_to_stablecoin(normalized)?;
    let pnl = self.exchange_context.rebalance_pnl_sell_side(
      &lst_price,
      out_amount,
      stablecoin_moved,
    )?;
    self.validate_pnl_settlement(&self.exchange_context, SUPPLY_FLOOR, pnl)?;
    let marginal_rate = atom_rate::<N6, N9>(
      self
        .exchange_context
        .rebalance_sell_marginal(&adjusted, usdc_price, normalized)?,
    );
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: USDC::MINT,
      fee_base: in_amount,
      marginal_rate,
    })
  }
}

impl<C: SolanaClock> TokenOperation<JITOSOL, USDC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<OperationOutput<N9, N6, N9>, CoreError> {
    self.rebalance_buy_quote::<JITOSOL>(in_amount)
  }
}

impl<C: SolanaClock> TokenOperation<HYLOSOL, USDC> for ProtocolState<C> {
  type FeeExp = N9;

  fn compute_output(
    &self,
    in_amount: UFix64<N9>,
  ) -> Result<OperationOutput<N9, N6, N9>, CoreError> {
    self.rebalance_buy_quote::<HYLOSOL>(in_amount)
  }
}

impl<C: SolanaClock> TokenOperation<USDC, JITOSOL> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N9, N6>, CoreError> {
    self.rebalance_sell_quote::<JITOSOL>(in_amount)
  }
}

impl<C: SolanaClock> TokenOperation<USDC, HYLOSOL> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N9, N6>, CoreError> {
    self.rebalance_sell_quote::<HYLOSOL>(in_amount)
  }
}

impl<C: SolanaClock> TokenOperation<CBBTC, USDC> for ProtocolState<C> {
  type FeeExp = N8;

  fn compute_output(
    &self,
    in_amount: UFix64<N8>,
  ) -> Result<OperationOutput<N8, N6, N8>, CoreError> {
    let exo = self.cbbtc_exchange_context();
    let btc_pair = &self.btc_pair_state;
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!btc_pair.paused, CoreError::PairPaused)?;
    gate(!self.usdc_exchange_state().paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      btc_pair.pool_drawdown.is_repaid(),
      CoreError::DrawdownNotRepaid,
    )?;
    gate(
      btc_pair.borrow_rate_harvest_epoch == exo.clock.epoch(),
      CoreError::BorrowRateHarvestNotRun,
    )?;
    gate(exo.rebalance_buy_active(), CoreError::OperationDisabled)?;
    let normalized: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    gate(
      normalized <= exo.rebalance_buy_target()?,
      CoreError::RebalanceBuyTargetExceeded,
    )?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let conversion = exo.rebalance_buy_conversion(usdc_price, normalized)?;
    let usdc_out: UFix64<N9> = conversion.collateral_to_usdc(normalized)?;
    let out_amount = usdc_out
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    gate(
      out_amount <= self.usdc_exchange_state().vault_balance,
      CoreError::InsufficientLiquidity,
    )?;
    let stablecoin_moved = self
      .usdc_exchange_state()
      .conversion()
      .withdrawal_to_stablecoin(usdc_out)?;
    let pnl = exo.rebalance_pnl_buy_side(normalized, stablecoin_moved)?;
    self.validate_pnl_settlement(exo, btc_pair.supply_floor, pnl)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: CBBTC::MINT,
      fee_base: in_amount,
      marginal_rate: atom_rate::<N8, N6>(
        exo.rebalance_buy_marginal(usdc_price, normalized)?,
      ),
    })
  }
}

impl<C: SolanaClock> TokenOperation<USDC, CBBTC> for ProtocolState<C> {
  type FeeExp = N6;

  fn compute_output(
    &self,
    in_amount: UFix64<N6>,
  ) -> Result<OperationOutput<N6, N8, N6>, CoreError> {
    let exo = self.cbbtc_exchange_context();
    let btc_pair = &self.btc_pair_state;
    gate(!self.protocol_paused, CoreError::ProtocolPaused)?;
    gate(!btc_pair.paused, CoreError::PairPaused)?;
    gate(!self.usdc_exchange_state().paused, CoreError::PairPaused)?;
    gate(in_amount > UFix64::zero(), CoreError::ZeroAmount)?;
    gate(
      btc_pair.pool_drawdown.is_repaid(),
      CoreError::DrawdownNotRepaid,
    )?;
    gate(
      btc_pair.borrow_rate_harvest_epoch == exo.clock.epoch(),
      CoreError::BorrowRateHarvestNotRun,
    )?;
    gate(exo.rebalance_sell_active(), CoreError::OperationDisabled)?;
    let normalized: UFix64<N9> = in_amount
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    let usdc_price = self.usdc_exchange_state().usdc_usd_price;
    let max_usdc_in =
      exo.max_rebalance_sell_usdc(usdc_price, btc_pair.supply_floor)?;
    gate(normalized <= max_usdc_in, CoreError::InsufficientLiquidity)?;
    let conversion = exo.rebalance_sell_conversion(usdc_price, normalized)?;
    let collateral_out: UFix64<N9> =
      conversion.usdc_to_collateral(normalized)?;
    let out_amount = collateral_out
      .checked_convert()
      .ok_or(CoreError::TokenAmountPrecision)?;
    let stablecoin_moved = self
      .usdc_exchange_state()
      .conversion()
      .deposit_to_stablecoin(normalized)?;
    let pnl = exo.rebalance_pnl_sell_side(collateral_out, stablecoin_moved)?;
    self.validate_pnl_settlement(exo, btc_pair.supply_floor, pnl)?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount: UFix64::zero(),
      fee_mint: USDC::MINT,
      fee_base: in_amount,
      marginal_rate: atom_rate::<N6, N8>(
        exo.rebalance_sell_marginal(usdc_price, normalized)?,
      ),
    })
  }
}
