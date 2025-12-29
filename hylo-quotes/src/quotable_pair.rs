use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fix::prelude::{UFix64, N6, N9};
use hylo_clients::prelude::{
  ExchangeClient, SimulatePrice, StabilityPoolClient,
};
use hylo_clients::protocol_state::ProtocolState;
use hylo_clients::transaction::{
  MintArgs, RedeemArgs, StabilityPoolArgs, SwapArgs,
};
use hylo_core::fee_controller::FeeExtract;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_mode::StabilityMode;
use hylo_core::stability_pool_math::{lp_token_nav, lp_token_out};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};

use crate::{LstProvider, QuoteAmounts, LST};

/// Trait indicating a token pair is quotable and can compute quotes.
#[async_trait]
pub trait QuotablePair<IN: TokenMint, OUT: TokenMint, C: SolanaClock>:
  private::Sealed
{
  /// Compute quote for this token pair from protocol state.
  ///
  /// # Errors
  /// Returns error if quote computation fails or pair is unsupported.
  fn quote_from_state(
    state: &ProtocolState<C>,
    amount_in: u64,
  ) -> Result<QuoteAmounts>;

  /// Simulates the transaction and extracts quote amounts (output + fees) from
  /// the event.
  ///
  /// This leverages the client's `SimulatePrice` implementation to perform the
  /// simulation and extract both output amounts and fees from the event.
  async fn simulate_quote(
    exchange: &ExchangeClient,
    stability: &StabilityPoolClient,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts>;
}

// ============================================================================
// Implementations for LST → HYUSD (mint stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuotablePair<L, HYUSD, C> for (L, HYUSD)
where
  ProtocolState<C>: LstProvider<L>,
{
  // async fn build_transaction_data(
  //   exchange: &ExchangeClient,
  //   _stability: &StabilityPoolClient,
  //   amount_in: u64,
  //   user: Pubkey,
  // ) -> Result<VersionedTransactionData> {
  //   let inputs = MintArgs {
  //     amount: UFix64::<N9>::new(amount_in),
  //     user,
  //     slippage_config: None,
  //   };
  //   <ExchangeClient as BuildTransactionData<L, HYUSD>>::build(exchange,
  // inputs)     .await
  // }

  fn quote_from_state(
    state: &ProtocolState<C>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
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

    let hyusd_out = {
      let converted = state
        .exchange_context
        .token_conversion(&lst_price)?
        .lst_to_token(amount_remaining, stablecoin_nav)?;
      state
        .exchange_context
        .validate_stablecoin_amount(converted)?
    };

    Ok(QuoteAmounts {
      amount_in: amount_in.bits,
      amount_out: hyusd_out.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
    })
  }

  async fn simulate_quote(
    exchange: &ExchangeClient,
    _stability: &StabilityPoolClient,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts> {
    let inputs = MintArgs {
      amount: UFix64::<N9>::new(amount_in),
      user,
      slippage_config: None,
    };

    let event = <ExchangeClient as SimulatePrice<L, HYUSD>>::simulate_event(
      exchange, user, inputs,
    )
    .await?;

    Ok(QuoteAmounts {
      amount_in,
      amount_out: event.minted.bits,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
    })
  }
}

// ============================================================================
// Implementations for HYUSD → LST (redeem stablecoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuotablePair<HYUSD, L, C> for (HYUSD, L)
where
  ProtocolState<C>: LstProvider<L>,
{
  // async fn build_transaction_data(
  //   exchange: &ExchangeClient,
  //   _stability: &StabilityPoolClient,
  //   amount_in: u64,
  //   user: Pubkey,
  // ) -> Result<VersionedTransactionData> {
  //   let inputs = RedeemArgs {
  //     amount: UFix64::<N6>::new(amount_in),
  //     user,
  //     slippage_config: None,
  //   };
  //   <ExchangeClient as BuildTransactionData<HYUSD, L>>::build(exchange,
  // inputs)     .await
  // }

  fn quote_from_state(
    state: &ProtocolState<C>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
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

    Ok(QuoteAmounts {
      amount_in: amount_in.bits,
      amount_out: amount_remaining.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
    })
  }

  async fn simulate_quote(
    exchange: &ExchangeClient,
    _stability: &StabilityPoolClient,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts> {
    let inputs = RedeemArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: None,
    };
    let event = <ExchangeClient as SimulatePrice<HYUSD, L>>::simulate_event(
      exchange, user, inputs,
    )
    .await?;

    Ok(QuoteAmounts {
      amount_in,
      amount_out: event.collateral_withdrawn.bits,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
    })
  }
}

// ============================================================================
// Implementations for LST → XSOL (mint levercoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuotablePair<L, XSOL, C> for (L, XSOL)
where
  ProtocolState<C>: LstProvider<L>,
{
  // async fn build_transaction_data(
  //   exchange: &ExchangeClient,
  //   _stability: &StabilityPoolClient,
  //   amount_in: u64,
  //   user: Pubkey,
  // ) -> Result<VersionedTransactionData> {
  //   let inputs = MintArgs {
  //     amount: UFix64::<N9>::new(amount_in),
  //     user,
  //     slippage_config: None,
  //   };
  //   <ExchangeClient as BuildTransactionData<L, XSOL>>::build(exchange,
  // inputs)     .await
  // }

  fn quote_from_state(
    state: &ProtocolState<C>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
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

    Ok(QuoteAmounts {
      amount_in: amount_in.bits,
      amount_out: xsol_out.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
    })
  }

  async fn simulate_quote(
    exchange: &ExchangeClient,
    _stability: &StabilityPoolClient,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts> {
    let inputs = MintArgs {
      amount: UFix64::<N9>::new(amount_in),
      user,
      slippage_config: None,
    };
    let event = <ExchangeClient as SimulatePrice<L, XSOL>>::simulate_event(
      exchange, user, inputs,
    )
    .await?;

    Ok(QuoteAmounts {
      amount_in,
      amount_out: event.minted.bits,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
    })
  }
}

// ============================================================================
// Implementations for XSOL → LST (redeem levercoin)
// ============================================================================

#[async_trait]
impl<L: LST, C: SolanaClock> QuotablePair<XSOL, L, C> for (XSOL, L)
where
  ProtocolState<C>: LstProvider<L>,
{
  // async fn build_transaction_data(
  //   exchange: &ExchangeClient,
  //   _stability: &StabilityPoolClient,
  //   amount_in: u64,
  //   user: Pubkey,
  // ) -> Result<VersionedTransactionData> {
  //   let inputs = RedeemArgs {
  //     amount: UFix64::<N6>::new(amount_in),
  //     user,
  //     slippage_config: None,
  //   };
  //   <ExchangeClient as BuildTransactionData<XSOL, L>>::build(exchange,
  // inputs)     .await
  // }

  fn quote_from_state(
    state: &ProtocolState<C>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
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

    Ok(QuoteAmounts {
      amount_in: amount_in.bits,
      amount_out: amount_remaining.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: L::MINT,
    })
  }

  async fn simulate_quote(
    exchange: &ExchangeClient,
    _stability: &StabilityPoolClient,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts> {
    let inputs = RedeemArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: None,
    };
    let event = <ExchangeClient as SimulatePrice<XSOL, L>>::simulate_event(
      exchange, user, inputs,
    )
    .await?;

    Ok(QuoteAmounts {
      amount_in,
      amount_out: event.collateral_withdrawn.bits,
      fee_amount: event.fees_deposited.bits,
      fee_mint: event.lst_mint,
    })
  }
}

// ============================================================================
// Implementations for HYUSD → XSOL (swap)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuotablePair<HYUSD, XSOL, C> for (HYUSD, XSOL) {
  // async fn build_transaction_data(
  //   exchange: &ExchangeClient,
  //   _stability: &StabilityPoolClient,
  //   amount_in: u64,
  //   user: Pubkey,
  // ) -> Result<VersionedTransactionData> {
  //   let inputs = SwapArgs {
  //     amount: UFix64::<N6>::new(amount_in),
  //     user,
  //     slippage_config: None,
  //   };
  //   <ExchangeClient as BuildTransactionData<HYUSD, XSOL>>::build(
  //     exchange, inputs,
  //   )
  //   .await
  // }

  fn quote_from_state(
    state: &ProtocolState<C>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
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

    Ok(QuoteAmounts {
      amount_in: amount_in.bits,
      amount_out: xsol_out.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: HYUSD::MINT,
    })
  }

  async fn simulate_quote(
    exchange: &ExchangeClient,
    _stability: &StabilityPoolClient,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts> {
    let inputs = SwapArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: None,
    };
    let event = <ExchangeClient as SimulatePrice<HYUSD, XSOL>>::simulate_event(
      exchange, user, inputs,
    )
    .await?;

    Ok(QuoteAmounts {
      amount_in,
      amount_out: event.levercoin_minted.bits,
      fee_amount: event.stablecoin_fees.bits,
      fee_mint: HYUSD::MINT,
    })
  }
}

// ============================================================================
// Implementations for XSOL → HYUSD (swap)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuotablePair<XSOL, HYUSD, C> for (XSOL, HYUSD) {
  // async fn build_transaction_data(
  //   exchange: &ExchangeClient,
  //   _stability: &StabilityPoolClient,
  //   amount_in: u64,
  //   user: Pubkey,
  // ) -> Result<VersionedTransactionData> {
  //   let inputs = SwapArgs {
  //     amount: UFix64::<N6>::new(amount_in),
  //     user,
  //     slippage_config: None,
  //   };
  //   <ExchangeClient as BuildTransactionData<XSOL, HYUSD>>::build(
  //     exchange, inputs,
  //   )
  //   .await
  // }

  fn quote_from_state(
    state: &ProtocolState<C>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
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

    Ok(QuoteAmounts {
      amount_in: amount_in.bits,
      amount_out: amount_remaining.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: HYUSD::MINT,
    })
  }

  async fn simulate_quote(
    exchange: &ExchangeClient,
    _stability: &StabilityPoolClient,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts> {
    let inputs = SwapArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: None,
    };
    let event = <ExchangeClient as SimulatePrice<XSOL, HYUSD>>::simulate_event(
      exchange, user, inputs,
    )
    .await?;

    Ok(QuoteAmounts {
      amount_in,
      amount_out: event.stablecoin_minted_user.bits,
      fee_amount: event.stablecoin_minted_fees.bits,
      fee_mint: HYUSD::MINT,
    })
  }
}

// ============================================================================
// Implementations for HYUSD → SHYUSD (stability pool deposit)
// ============================================================================

#[async_trait]
impl<C: SolanaClock> QuotablePair<HYUSD, SHYUSD, C> for (HYUSD, SHYUSD) {
  // async fn build_transaction_data(
  //   _exchange: &ExchangeClient,
  //   stability: &StabilityPoolClient,
  //   amount_in: u64,
  //   user: Pubkey,
  // ) -> Result<VersionedTransactionData> {
  //   let inputs = StabilityPoolArgs {
  //     amount: UFix64::<N6>::new(amount_in),
  //     user,
  //   };
  //   <StabilityPoolClient as BuildTransactionData<HYUSD, SHYUSD>>::build(
  //     stability, inputs,
  //   )
  //   .await
  // }

  fn quote_from_state(
    state: &ProtocolState<C>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    let amount_in = UFix64::<N6>::new(amount_in);

    let shyusd_nav = lp_token_nav(
      state.exchange_context.stablecoin_nav()?,
      UFix64::new(state.hyusd_pool.amount),
      state.exchange_context.levercoin_mint_nav()?,
      UFix64::new(state.xsol_pool.amount),
      UFix64::new(state.shyusd_mint.supply),
    )?;

    let shyusd_out = lp_token_out(amount_in, shyusd_nav)?;

    Ok(QuoteAmounts {
      amount_in: amount_in.bits,
      amount_out: shyusd_out.bits,
      fee_amount: 0,
      fee_mint: HYUSD::MINT,
    })
  }

  async fn simulate_quote(
    _exchange: &ExchangeClient,
    stability: &StabilityPoolClient,
    amount_in: u64,
    user: Pubkey,
  ) -> Result<QuoteAmounts> {
    let inputs = StabilityPoolArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
    };
    let event =
      <StabilityPoolClient as SimulatePrice<HYUSD, SHYUSD>>::simulate_event(
        stability, user, inputs,
      )
      .await?;

    Ok(QuoteAmounts {
      amount_in,
      amount_out: event.lp_token_minted.bits,
      fee_amount: 0, // UserDepositEvent has no fees
      fee_mint: HYUSD::MINT,
    })
  }
}

mod private {
  pub trait Sealed {}
  impl<IN: super::TokenMint, OUT: super::TokenMint> Sealed for (IN, OUT) {}
}
