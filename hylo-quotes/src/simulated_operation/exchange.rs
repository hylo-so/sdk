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
use crate::token_operation::{
  LstSwapOperationOutput, MintOperationOutput, RedeemOperationOutput,
  SwapOperationOutput,
};
use crate::{Local, LST};

/// Mint stablecoin from LST.
impl<L: LST + Local> SimulatedOperation<L, HYUSD> for ExchangeClient {
  type FeeExp = N9;
  type Event = MintStablecoinEventV2;

  fn from_event(event: &Self::Event) -> Result<MintOperationOutput> {
    let out_amount: UFix64<N6> = UFix64::new(event.minted.bits);
    let fee_amount: UFix64<N9> = UFix64::new(event.fees_deposited.bits);
    let collateral_deposited: UFix64<N9> =
      UFix64::new(event.collateral_deposited.bits);
    let fee_base: UFix64<N9> = collateral_deposited
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(MintOperationOutput {
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
  type FeeExp = N9;
  type Event = RedeemStablecoinEventV2;

  fn from_event(event: &Self::Event) -> Result<RedeemOperationOutput> {
    let in_amount: UFix64<N6> = UFix64::new(event.redeemed.bits);
    let out_amount: UFix64<N9> = UFix64::new(event.collateral_withdrawn.bits);
    let fee_amount: UFix64<N9> = UFix64::new(event.fees_deposited.bits);
    let fee_base: UFix64<N9> = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(RedeemOperationOutput {
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
  type FeeExp = N9;
  type Event = MintLevercoinEventV2;

  fn from_event(event: &Self::Event) -> Result<MintOperationOutput> {
    let out_amount: UFix64<N6> = UFix64::new(event.minted.bits);
    let fee_amount: UFix64<N9> = UFix64::new(event.fees_deposited.bits);
    let collateral_deposited: UFix64<N9> =
      UFix64::new(event.collateral_deposited.bits);
    let fee_base: UFix64<N9> = collateral_deposited
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(MintOperationOutput {
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
  type FeeExp = N9;
  type Event = RedeemLevercoinEventV2;

  fn from_event(event: &Self::Event) -> Result<RedeemOperationOutput> {
    let in_amount: UFix64<N6> = UFix64::new(event.redeemed.bits);
    let out_amount: UFix64<N9> = UFix64::new(event.collateral_withdrawn.bits);
    let fee_amount: UFix64<N9> = UFix64::new(event.fees_deposited.bits);
    let fee_base: UFix64<N9> = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(RedeemOperationOutput {
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

  fn from_event(event: &Self::Event) -> Result<SwapOperationOutput> {
    let stablecoin_burned: UFix64<N6> =
      UFix64::new(event.stablecoin_burned.bits);
    let out_amount: UFix64<N6> = UFix64::new(event.levercoin_minted.bits);
    let fee_amount: UFix64<N6> = UFix64::new(event.stablecoin_fees.bits);
    let fee_base: UFix64<N6> = stablecoin_burned
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(SwapOperationOutput {
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

  fn from_event(event: &Self::Event) -> Result<SwapOperationOutput> {
    let in_amount: UFix64<N6> = UFix64::new(event.levercoin_burned.bits);
    let out_amount: UFix64<N6> = UFix64::new(event.stablecoin_minted_user.bits);
    let fee_amount: UFix64<N6> = UFix64::new(event.stablecoin_minted_fees.bits);
    let fee_base: UFix64<N6> = out_amount
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(SwapOperationOutput {
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
  type FeeExp = N9;
  type Event = SwapLstEventV0;

  fn from_event(event: &Self::Event) -> Result<LstSwapOperationOutput> {
    let in_amount: UFix64<N9> = UFix64::new(event.lst_a_in.bits);
    let out_amount: UFix64<N9> = UFix64::new(event.lst_b_out.bits);
    let fee_amount: UFix64<N9> = UFix64::new(event.lst_a_fees_extracted.bits);
    Ok(LstSwapOperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: L1::MINT,
      fee_base: in_amount,
    })
  }
}
