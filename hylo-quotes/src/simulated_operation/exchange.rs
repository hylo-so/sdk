//! `SimulatedOperation` implementations for exchange pairs.

use anyhow::{Context, Result};
use fix::prelude::*;
use hylo_clients::prelude::ExchangeClient;
use hylo_idl::exchange::events::{
  MintLevercoinEventV2, MintStablecoinEventV2, RedeemLevercoinEventV2,
  RedeemStablecoinEventV2, SwapLeverToStableEventV1, SwapLstEventV0,
  SwapStableToLeverEventV1,
};
use hylo_idl::tokens::{TokenMint, HYUSD, XSOL};

use crate::simulated_operation::SimulatedOperation;
use crate::token_operation::OperationOutput;
use crate::{Local, LST};

/// Mint stablecoin from LST.
impl<L: LST + Local> SimulatedOperation<L, HYUSD> for ExchangeClient {
  type FeeExp = L::Exp;
  type Event = MintStablecoinEventV2;

  fn from_event(
    event: &Self::Event,
  ) -> Result<OperationOutput<L::Exp, N6, L::Exp>> {
    let out_amount = UFix64::new(event.minted.bits);
    let fee_amount = UFix64::new(event.fees_deposited.bits);
    let collateral_deposited = UFix64::new(event.collateral_deposited.bits);
    // fee_base = original input = collateral_deposited + fees_deposited
    let fee_base = collateral_deposited
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(OperationOutput {
      in_amount: fee_base,
      out_amount,
      fee_amount,
      fee_mint: L::MINT,
      fee_base,
    })
  }
}

/// Redeem stablecoin for LST.
impl<L: LST + Local> SimulatedOperation<HYUSD, L> for ExchangeClient {
  type FeeExp = L::Exp;
  type Event = RedeemStablecoinEventV2;

  fn from_event(
    event: &Self::Event,
  ) -> Result<OperationOutput<N6, L::Exp, L::Exp>> {
    let in_amount = UFix64::new(event.redeemed.bits);
    let out_amount = UFix64::new(event.collateral_withdrawn.bits);
    let fee_amount = UFix64::new(event.fees_deposited.bits);
    // fee_base = pre-fee LST output = collateral_withdrawn + fees_deposited
    let fee_base = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: L::MINT,
      fee_base,
    })
  }
}

/// Mint levercoin from LST.
impl<L: LST + Local> SimulatedOperation<L, XSOL> for ExchangeClient {
  type FeeExp = L::Exp;
  type Event = MintLevercoinEventV2;

  fn from_event(
    event: &Self::Event,
  ) -> Result<OperationOutput<L::Exp, N6, L::Exp>> {
    let out_amount = UFix64::new(event.minted.bits);
    let fee_amount = UFix64::new(event.fees_deposited.bits);
    let collateral_deposited = UFix64::new(event.collateral_deposited.bits);
    // fee_base = original input = collateral_deposited + fees_deposited
    let fee_base = collateral_deposited
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(OperationOutput {
      in_amount: fee_base,
      out_amount,
      fee_amount,
      fee_mint: L::MINT,
      fee_base,
    })
  }
}

/// Redeem levercoin for LST.
impl<L: LST + Local> SimulatedOperation<XSOL, L> for ExchangeClient {
  type FeeExp = L::Exp;
  type Event = RedeemLevercoinEventV2;

  fn from_event(
    event: &Self::Event,
  ) -> Result<OperationOutput<N6, L::Exp, L::Exp>> {
    let in_amount = UFix64::new(event.redeemed.bits);
    let out_amount = UFix64::new(event.collateral_withdrawn.bits);
    let fee_amount = UFix64::new(event.fees_deposited.bits);
    // fee_base = pre-fee LST output = collateral_withdrawn + fees_deposited
    let fee_base = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: L::MINT,
      fee_base,
    })
  }
}

/// Swap stablecoin to levercoin.
impl SimulatedOperation<HYUSD, XSOL> for ExchangeClient {
  type FeeExp = N6;
  type Event = SwapStableToLeverEventV1;

  fn from_event(event: &Self::Event) -> Result<OperationOutput<N6, N6, N6>> {
    let stablecoin_burned = UFix64::new(event.stablecoin_burned.bits);
    let out_amount = UFix64::new(event.levercoin_minted.bits);
    let fee_amount = UFix64::new(event.stablecoin_fees.bits);
    // fee_base = original input = stablecoin_burned + stablecoin_fees
    let fee_base = stablecoin_burned
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(OperationOutput {
      in_amount: fee_base,
      out_amount,
      fee_amount,
      fee_mint: HYUSD::MINT,
      fee_base,
    })
  }
}

/// Swap levercoin to stablecoin.
impl SimulatedOperation<XSOL, HYUSD> for ExchangeClient {
  type FeeExp = N6;
  type Event = SwapLeverToStableEventV1;

  fn from_event(event: &Self::Event) -> Result<OperationOutput<N6, N6, N6>> {
    let in_amount = UFix64::new(event.levercoin_burned.bits);
    let out_amount = UFix64::new(event.stablecoin_minted_user.bits);
    let fee_amount = UFix64::new(event.stablecoin_minted_fees.bits);
    // fee_base = total minted = stablecoin_minted_user + stablecoin_minted_fees
    let fee_base = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: HYUSD::MINT,
      fee_base,
    })
  }
}

/// Swap between LSTs.
impl<L1: LST + Local, L2: LST + Local> SimulatedOperation<L1, L2>
  for ExchangeClient
{
  type FeeExp = L1::Exp;
  type Event = SwapLstEventV0;

  fn from_event(
    event: &Self::Event,
  ) -> Result<OperationOutput<L1::Exp, L2::Exp, L1::Exp>> {
    let in_amount = UFix64::new(event.lst_a_in.bits);
    let out_amount = UFix64::new(event.lst_b_out.bits);
    let fee_amount = UFix64::new(event.lst_a_fees_extracted.bits);
    // fee_base = lst_a_in (directly available in event)
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: L1::MINT,
      fee_base: in_amount,
    })
  }
}
