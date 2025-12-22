//! Type-safe quote computation using generics matching the SDK pattern.

use anyhow::{anyhow, Result};
use hylo_clients::prelude::*;
use hylo_clients::protocol_state::ProtocolState;
use hylo_core::fee_controller::FeeExtract;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_mode::StabilityMode;
use hylo_core::stability_pool_math::{lp_token_nav, lp_token_out};
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::{QuoteAmounts, SupportedPair};

/// Trait for computing quotes for token pairs.
pub trait QuoteComputer<IN: TokenMint, OUT: TokenMint, S: SolanaClock>:
  Send + Sync
where
  (IN, OUT): SupportedPair<IN, OUT>,
{
  /// Compute quote for a token pair operation.
  ///
  /// # Errors
  /// Returns error if quote computation fails or pair is unsupported.
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount: u64,
  ) -> Result<QuoteAmounts>;
}

/// Quote computer for Hylo protocol token pairs.
pub struct HyloQuoteComputer;

impl HyloQuoteComputer {
  #[must_use]
  pub fn new() -> Self {
    Self
  }
}

impl Default for HyloQuoteComputer {
  fn default() -> Self {
    Self::new()
  }
}

impl<S: SolanaClock> QuoteComputer<JITOSOL, HYUSD, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    if state.exchange_context.stability_mode > StabilityMode::Mode1 {
      return Err(anyhow!(
        "Mint operations disabled in current stability mode"
      ));
    }

    let amount_in = UFix64::<N9>::new(amount_in);
    let lst_price = state.jitosol_header.price_sol.into();

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
      fee_mint: state.jitosol_header.mint,
    })
  }
}

// ============================================================================
// Implementations for HYUSD → JITOSOL (redeem stablecoin)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<HYUSD, JITOSOL, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    let amount_in = UFix64::<N6>::new(amount_in);
    let lst_price = state.jitosol_header.price_sol.into();

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
      fee_mint: state.jitosol_header.mint,
    })
  }
}

// ============================================================================
// Implementations for HYLOSOL → HYUSD (mint stablecoin)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<HYLOSOL, HYUSD, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    if state.exchange_context.stability_mode > StabilityMode::Mode1 {
      return Err(anyhow!(
        "Mint operations disabled in current stability mode"
      ));
    }

    let amount_in = UFix64::<N9>::new(amount_in);
    let lst_price = state.hylosol_header.price_sol.into();

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
      state.exchange_context.validate_stablecoin_amount(converted)
    }?;

    Ok(QuoteAmounts {
      amount_in: amount_in.bits,
      amount_out: hyusd_out.bits,
      fee_amount: fees_extracted.bits,
      fee_mint: state.hylosol_header.mint,
    })
  }
}

// ============================================================================
// Implementations for HYUSD → HYLOSOL (redeem stablecoin)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<HYUSD, HYLOSOL, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    let amount_in = UFix64::<N6>::new(amount_in);
    let lst_price = state.hylosol_header.price_sol.into();

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
      fee_mint: state.hylosol_header.mint,
    })
  }
}

// ============================================================================
// Implementations for JITOSOL → XSOL (mint levercoin)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<JITOSOL, XSOL, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    if state.exchange_context.stability_mode == StabilityMode::Depeg {
      return Err(anyhow!(
        "Levercoin mint disabled in current stability mode"
      ));
    }

    let amount_in = UFix64::<N9>::new(amount_in);
    let lst_price = state.jitosol_header.price_sol.into();

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
      fee_mint: state.jitosol_header.mint,
    })
  }
}

// ============================================================================
// Implementations for XSOL → JITOSOL (redeem levercoin)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<XSOL, JITOSOL, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    if state.exchange_context.stability_mode == StabilityMode::Depeg {
      return Err(anyhow!(
        "Levercoin redemption disabled in current stability mode"
      ));
    }

    let amount_in = UFix64::<N6>::new(amount_in);
    let lst_price = state.jitosol_header.price_sol.into();

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
      fee_mint: state.jitosol_header.mint,
    })
  }
}

// ============================================================================
// Implementations for HYLOSOL → XSOL (mint levercoin)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<HYLOSOL, XSOL, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    if state.exchange_context.stability_mode == StabilityMode::Depeg {
      return Err(anyhow!(
        "Levercoin mint disabled in current stability mode"
      ));
    }

    let amount_in = UFix64::<N9>::new(amount_in);
    let lst_price = state.hylosol_header.price_sol.into();

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
      fee_mint: state.hylosol_header.mint,
    })
  }
}

// ============================================================================
// Implementations for XSOL → HYLOSOL (redeem levercoin)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<XSOL, HYLOSOL, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
    amount_in: u64,
  ) -> Result<QuoteAmounts> {
    if state.exchange_context.stability_mode == StabilityMode::Depeg {
      return Err(anyhow!(
        "Levercoin redemption disabled in current stability mode"
      ));
    }

    let amount_in = UFix64::<N6>::new(amount_in);
    let lst_price = state.hylosol_header.price_sol.into();

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
      fee_mint: state.hylosol_header.mint,
    })
  }
}

// ============================================================================
// Implementations for HYUSD → XSOL (swap)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<HYUSD, XSOL, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
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
}

// ============================================================================
// Implementations for XSOL → HYUSD (swap)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<XSOL, HYUSD, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
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
}

// ============================================================================
// Implementations for HYUSD → SHYUSD (stability pool deposit)
// ============================================================================

impl<S: SolanaClock> QuoteComputer<HYUSD, SHYUSD, S> for HyloQuoteComputer {
  fn compute_quote(
    &self,
    state: &ProtocolState<S>,
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
}
