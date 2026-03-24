//! `SimulatedOperation` implementations for exchange pairs.

use anyhow::{Context, Result};
use fix::prelude::*;
use hylo_clients::prelude::ExchangeClient;
use hylo_idl::exchange::events::{
  ConvertLeverToStableExoEvent, ConvertLeverToStableLstEvent,
  ConvertStableToLeverExoEvent, ConvertStableToLeverLstEvent,
  MintLevercoinExoEvent, MintLevercoinLstEvent, MintStablecoinExoEvent,
  MintStablecoinLstEvent, MintStablecoinUsdcEvent, RedeemLevercoinExoEvent,
  RedeemLevercoinLstEvent, RedeemStablecoinExoEvent, RedeemStablecoinLstEvent,
  RedeemStablecoinUsdcEvent, SwapLstToLstEvent,
};
use hylo_idl::tokens::{TokenMint, CBBTC, HYUSD, USDC, XBTC, XSOL};

use crate::simulated_operation::SimulatedOperation;
use crate::token_operation::{
  LstSwapOperationOutput, MintOperationOutput, OperationOutput,
  RedeemOperationOutput, SwapOperationOutput,
};
use crate::{Local, LST};

/// Mint stablecoin from LST.
impl<L: LST + Local> SimulatedOperation<L, HYUSD> for ExchangeClient {
  type FeeExp = N9;
  type Event = MintStablecoinLstEvent;

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
  type Event = RedeemStablecoinLstEvent;

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
  type Event = MintLevercoinLstEvent;

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
  type Event = RedeemLevercoinLstEvent;

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

/// Convert stablecoin to levercoin.
impl SimulatedOperation<HYUSD, XSOL> for ExchangeClient {
  type FeeExp = N6;
  type Event = ConvertStableToLeverLstEvent;

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

/// Convert levercoin to stablecoin.
impl SimulatedOperation<XSOL, HYUSD> for ExchangeClient {
  type FeeExp = N6;
  type Event = ConvertLeverToStableLstEvent;

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
  type Event = SwapLstToLstEvent;

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

// ============================================================================
// Exo / USDC simulated operations
// ============================================================================

/// Mint stablecoin from USDC.
impl SimulatedOperation<USDC, HYUSD> for ExchangeClient {
  type FeeExp = N6;
  type Event = MintStablecoinUsdcEvent;

  fn extract_output(event: &Self::Event) -> Result<SwapOperationOutput> {
    let usdc_deposited: UFix64<N6> =
      event.usdc_deposited.try_into()?;
    let out_amount: UFix64<N6> =
      event.stablecoin_minted.try_into()?;
    let fee_amount: UFix64<N6> = event.usdc_fees.try_into()?;
    let fee_base = usdc_deposited
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(SwapOperationOutput {
      in_amount: fee_base,
      out_amount,
      fee_amount,
      fee_mint: USDC::MINT,
      fee_base,
    })
  }
}

/// Redeem stablecoin to USDC.
impl SimulatedOperation<HYUSD, USDC> for ExchangeClient {
  type FeeExp = N6;
  type Event = RedeemStablecoinUsdcEvent;

  fn extract_output(event: &Self::Event) -> Result<SwapOperationOutput> {
    let in_amount: UFix64<N6> =
      event.stablecoin_burned.try_into()?;
    let usdc_withdrawn: UFix64<N6> =
      event.usdc_withdrawn.try_into()?;
    let fee_amount: UFix64<N6> =
      event.stablecoin_fees.try_into()?;
    let fee_base = usdc_withdrawn
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    Ok(SwapOperationOutput {
      in_amount,
      out_amount: usdc_withdrawn,
      fee_amount,
      fee_mint: USDC::MINT,
      fee_base,
    })
  }
}

/// Mint stablecoin from exo collateral (cbBTC -> HYUSD).
impl SimulatedOperation<CBBTC, HYUSD> for ExchangeClient {
  type FeeExp = N9;
  type Event = MintStablecoinExoEvent;

  fn extract_output(
    event: &Self::Event,
  ) -> Result<OperationOutput<N8, N6, N9>> {
    let out_amount: UFix64<N6> = event.minted.try_into()?;
    let fee_amount: UFix64<N9> = event.fees_deposited.try_into()?;
    let collateral_deposited: UFix64<N9> =
      event.collateral_deposited.try_into()?;
    let fee_base = collateral_deposited
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    let in_amount: UFix64<N8> = fee_base
      .checked_convert()
      .context("N9->N8 conversion")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: CBBTC::MINT,
      fee_base,
    })
  }
}

/// Redeem stablecoin for exo collateral (HYUSD -> cbBTC).
impl SimulatedOperation<HYUSD, CBBTC> for ExchangeClient {
  type FeeExp = N9;
  type Event = RedeemStablecoinExoEvent;

  fn extract_output(
    event: &Self::Event,
  ) -> Result<OperationOutput<N6, N8, N9>> {
    let in_amount: UFix64<N6> = event.redeemed.try_into()?;
    let collateral_withdrawn: UFix64<N9> =
      event.collateral_withdrawn.try_into()?;
    let fee_amount: UFix64<N9> = event.fees_deposited.try_into()?;
    let fee_base = collateral_withdrawn
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    let out_amount: UFix64<N8> = collateral_withdrawn
      .checked_convert()
      .context("N9->N8 conversion")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: CBBTC::MINT,
      fee_base,
    })
  }
}

/// Mint levercoin from exo collateral (cbBTC -> xBTC).
impl SimulatedOperation<CBBTC, XBTC> for ExchangeClient {
  type FeeExp = N9;
  type Event = MintLevercoinExoEvent;

  fn extract_output(
    event: &Self::Event,
  ) -> Result<OperationOutput<N8, N6, N9>> {
    let out_amount: UFix64<N6> = event.minted.try_into()?;
    let fee_amount: UFix64<N9> = event.fees_deposited.try_into()?;
    let collateral_deposited: UFix64<N9> =
      event.collateral_deposited.try_into()?;
    let fee_base = collateral_deposited
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    let in_amount: UFix64<N8> = fee_base
      .checked_convert()
      .context("N9->N8 conversion")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: CBBTC::MINT,
      fee_base,
    })
  }
}

/// Redeem levercoin for exo collateral (xBTC -> cbBTC).
impl SimulatedOperation<XBTC, CBBTC> for ExchangeClient {
  type FeeExp = N9;
  type Event = RedeemLevercoinExoEvent;

  fn extract_output(
    event: &Self::Event,
  ) -> Result<OperationOutput<N6, N8, N9>> {
    let in_amount: UFix64<N6> = event.redeemed.try_into()?;
    let collateral_withdrawn: UFix64<N9> =
      event.collateral_withdrawn.try_into()?;
    let fee_amount: UFix64<N9> = event.fees_deposited.try_into()?;
    let fee_base = collateral_withdrawn
      .checked_add(&fee_amount)
      .context("fee_base overflow")?;
    let out_amount: UFix64<N8> = collateral_withdrawn
      .checked_convert()
      .context("N9->N8 conversion")?;
    Ok(OperationOutput {
      in_amount,
      out_amount,
      fee_amount,
      fee_mint: CBBTC::MINT,
      fee_base,
    })
  }
}

/// Convert stablecoin to exo levercoin (HYUSD -> xBTC).
impl SimulatedOperation<HYUSD, XBTC> for ExchangeClient {
  type FeeExp = N6;
  type Event = ConvertStableToLeverExoEvent;

  fn extract_output(event: &Self::Event) -> Result<SwapOperationOutput> {
    let stablecoin_burned: UFix64<N6> =
      event.stablecoin_burned.try_into()?;
    let out_amount: UFix64<N6> =
      event.levercoin_minted.try_into()?;
    let fee_amount: UFix64<N6> =
      event.stablecoin_fees.try_into()?;
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

/// Convert exo levercoin to stablecoin (xBTC -> HYUSD).
impl SimulatedOperation<XBTC, HYUSD> for ExchangeClient {
  type FeeExp = N6;
  type Event = ConvertLeverToStableExoEvent;

  fn extract_output(event: &Self::Event) -> Result<SwapOperationOutput> {
    let in_amount: UFix64<N6> =
      event.levercoin_burned.try_into()?;
    let out_amount: UFix64<N6> =
      event.stablecoin_minted_user.try_into()?;
    let fee_amount: UFix64<N6> =
      event.stablecoin_minted_fees.try_into()?;
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
