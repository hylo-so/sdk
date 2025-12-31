//! Quote provider with mint pair matching.

use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::quote_metadata::{Operation, QuoteMetadata};
use crate::quote_strategy::QuoteStrategy;
use crate::syntax_helpers::get_quote;
use crate::Quote;

/// Provider that matches mint pairs and fetches quotes via a strategy.
pub struct QuoteProvider<S> {
  strategy: S,
}

impl<S> QuoteProvider<S>
where
  // Exchange operations
  S: QuoteStrategy<JITOSOL, HYUSD, Clock>
    + QuoteStrategy<HYUSD, JITOSOL, Clock>
    + QuoteStrategy<HYLOSOL, HYUSD, Clock>
    + QuoteStrategy<HYUSD, HYLOSOL, Clock>
    + QuoteStrategy<JITOSOL, XSOL, Clock>
    + QuoteStrategy<XSOL, JITOSOL, Clock>
    + QuoteStrategy<HYLOSOL, XSOL, Clock>
    + QuoteStrategy<XSOL, HYLOSOL, Clock>
    + QuoteStrategy<HYUSD, XSOL, Clock>
    + QuoteStrategy<XSOL, HYUSD, Clock>
    // Stability pool operations
    + QuoteStrategy<HYUSD, SHYUSD, Clock>
    + QuoteStrategy<SHYUSD, HYUSD, Clock>,
{
  #[must_use]
  pub fn new(strategy: S) -> Self {
    Self { strategy }
  }

  /// Fetch a quote for a mint pair
  ///
  /// # Errors
  /// Returns error if the mint pair is unsupported or if quote fetching fails.
  #[allow(clippy::too_many_lines)]
  pub async fn fetch_quote(
    &self,
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> anyhow::Result<(Quote, QuoteMetadata)> {
    let (operation, description, quote_result) = match (input_mint, output_mint)
    {
      (JITOSOL::MINT, HYUSD::MINT) => (
        Operation::MintStablecoin,
        "Mint hyUSD with JitoSOL",
        get_quote::<S, JITOSOL, HYUSD, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYUSD::MINT, JITOSOL::MINT) => (
        Operation::RedeemStablecoin,
        "Redeem hyUSD for JitoSOL",
        get_quote::<S, HYUSD, JITOSOL, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYLOSOL::MINT, HYUSD::MINT) => (
        Operation::MintStablecoin,
        "Mint hyUSD with hyloSOL",
        get_quote::<S, HYLOSOL, HYUSD, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYUSD::MINT, HYLOSOL::MINT) => (
        Operation::RedeemStablecoin,
        "Redeem hyUSD for hyloSOL",
        get_quote::<S, HYUSD, HYLOSOL, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (JITOSOL::MINT, XSOL::MINT) => (
        Operation::MintLevercoin,
        "Mint xSOL with JitoSOL",
        get_quote::<S, JITOSOL, XSOL, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (XSOL::MINT, JITOSOL::MINT) => (
        Operation::RedeemLevercoin,
        "Redeem xSOL for JitoSOL",
        get_quote::<S, XSOL, JITOSOL, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYLOSOL::MINT, XSOL::MINT) => (
        Operation::MintLevercoin,
        "Mint xSOL with hyloSOL",
        get_quote::<S, HYLOSOL, XSOL, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (XSOL::MINT, HYLOSOL::MINT) => (
        Operation::RedeemLevercoin,
        "Redeem xSOL for hyloSOL",
        get_quote::<S, XSOL, HYLOSOL, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYUSD::MINT, XSOL::MINT) => (
        Operation::SwapStableToLever,
        "Swap hyUSD to xSOL",
        get_quote::<S, HYUSD, XSOL, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (XSOL::MINT, HYUSD::MINT) => (
        Operation::SwapLeverToStable,
        "Swap xSOL to hyUSD",
        get_quote::<S, XSOL, HYUSD, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYUSD::MINT, SHYUSD::MINT) => (
        Operation::DepositToStabilityPool,
        "Deposit hyUSD to Stability Pool",
        get_quote::<S, HYUSD, SHYUSD, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (SHYUSD::MINT, HYUSD::MINT) => (
        Operation::WithdrawFromStabilityPool,
        "Withdraw hyUSD from Stability Pool",
        get_quote::<S, SHYUSD, HYUSD, Clock>(
          &self.strategy,
          amount_in,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      _ => return Err(anyhow::anyhow!("Unsupported pair")),
    };

    let quote = quote_result?;
    let metadata = QuoteMetadata::new(operation, description);

    Ok((quote, metadata))
  }
}
