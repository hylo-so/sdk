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

  fn extract_output(event: &Self::Event) -> Result<MintOperationOutput> {
    let out_amount: UFix64<N6> = event.minted.try_into()?;
    let fee_amount: UFix64<N9> = event.fees_deposited.try_into()?;
    let collateral_deposited: UFix64<N9> =
      event.collateral_deposited.try_into()?;
    let fee_base = collateral_deposited
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

  fn extract_output(event: &Self::Event) -> Result<RedeemOperationOutput> {
    let in_amount: UFix64<N6> = event.redeemed.try_into()?;
    let out_amount: UFix64<N9> = event.collateral_withdrawn.try_into()?;
    let fee_amount: UFix64<N9> = event.fees_deposited.try_into()?;
    let fee_base = out_amount
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

  fn extract_output(event: &Self::Event) -> Result<MintOperationOutput> {
    let out_amount: UFix64<N6> = event.minted.try_into()?;
    let fee_amount: UFix64<N9> = event.fees_deposited.try_into()?;
    let collateral_deposited: UFix64<N9> =
      event.collateral_deposited.try_into()?;
    let fee_base = collateral_deposited
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

  fn extract_output(event: &Self::Event) -> Result<RedeemOperationOutput> {
    let in_amount: UFix64<N6> = event.redeemed.try_into()?;
    let out_amount: UFix64<N9> = event.collateral_withdrawn.try_into()?;
    let fee_amount: UFix64<N9> = event.fees_deposited.try_into()?;
    let fee_base = out_amount
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

  fn extract_output(event: &Self::Event) -> Result<SwapOperationOutput> {
    let stablecoin_burned: UFix64<N6> = event.stablecoin_burned.try_into()?;
    let out_amount: UFix64<N6> = event.levercoin_minted.try_into()?;
    let fee_amount: UFix64<N6> = event.stablecoin_fees.try_into()?;
    let fee_base = stablecoin_burned
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

  fn extract_output(event: &Self::Event) -> Result<SwapOperationOutput> {
    let in_amount: UFix64<N6> = event.levercoin_burned.try_into()?;
    let out_amount: UFix64<N6> = event.stablecoin_minted_user.try_into()?;
    let fee_amount: UFix64<N6> = event.stablecoin_minted_fees.try_into()?;
    let fee_base = out_amount
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

  fn extract_output(event: &Self::Event) -> Result<LstSwapOperationOutput> {
    let in_amount: UFix64<N9> = event.lst_a_in.try_into()?;
    let out_amount: UFix64<N9> = event.lst_b_out.try_into()?;
    let fee_amount: UFix64<N9> = event.lst_a_fees_extracted.try_into()?;
    Ok(LstSwapOperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: L1::MINT,
      fee_base: in_amount,
    })
  }
}
