//! Quote provider with mint matching logic

use anchor_lang::prelude::Pubkey;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::quote_metadata::{Operation, QuoteMetadata};
use crate::quote_strategy::QuoteStrategy;
use crate::ExecutableQuote;

/// Provider that matches mint pairs and fetches quotes
pub struct QuoteProvider<S: QuoteStrategy> {
  strategy: S,
}

impl<S: QuoteStrategy> QuoteProvider<S> {
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
    amount: u64,
    user: Pubkey,
    slippage_bps: u16,
  ) -> anyhow::Result<(ExecutableQuote, QuoteMetadata)> {
    let (operation, description, quote_result) = match (input_mint, output_mint)
    {
      (JITOSOL::MINT, HYUSD::MINT) => (
        Operation::MintStablecoin,
        "Mint hyUSD with JitoSOL",
        self
          .strategy
          .fetch_quote::<JITOSOL, HYUSD>(amount, user, slippage_bps)
          .await,
      ),
      (HYUSD::MINT, JITOSOL::MINT) => (
        Operation::RedeemStablecoin,
        "Redeem hyUSD for JitoSOL",
        self
          .strategy
          .fetch_quote::<HYUSD, JITOSOL>(amount, user, slippage_bps)
          .await,
      ),
      (HYLOSOL::MINT, HYUSD::MINT) => (
        Operation::MintStablecoin,
        "Mint hyUSD with hyloSOL",
        self
          .strategy
          .fetch_quote::<HYLOSOL, HYUSD>(amount, user, slippage_bps)
          .await,
      ),
      (HYUSD::MINT, HYLOSOL::MINT) => (
        Operation::RedeemStablecoin,
        "Redeem hyUSD for hyloSOL",
        self
          .strategy
          .fetch_quote::<HYUSD, HYLOSOL>(amount, user, slippage_bps)
          .await,
      ),
      (JITOSOL::MINT, XSOL::MINT) => (
        Operation::MintLevercoin,
        "Mint xSOL with JitoSOL",
        self
          .strategy
          .fetch_quote::<JITOSOL, XSOL>(amount, user, slippage_bps)
          .await,
      ),
      (XSOL::MINT, JITOSOL::MINT) => (
        Operation::RedeemLevercoin,
        "Redeem xSOL for JitoSOL",
        self
          .strategy
          .fetch_quote::<XSOL, JITOSOL>(amount, user, slippage_bps)
          .await,
      ),
      (HYLOSOL::MINT, XSOL::MINT) => (
        Operation::MintLevercoin,
        "Mint xSOL with hyloSOL",
        self
          .strategy
          .fetch_quote::<HYLOSOL, XSOL>(amount, user, slippage_bps)
          .await,
      ),
      (XSOL::MINT, HYLOSOL::MINT) => (
        Operation::RedeemLevercoin,
        "Redeem xSOL for hyloSOL",
        self
          .strategy
          .fetch_quote::<XSOL, HYLOSOL>(amount, user, slippage_bps)
          .await,
      ),
      (HYUSD::MINT, XSOL::MINT) => (
        Operation::SwapStableToLever,
        "Swap hyUSD to xSOL",
        self
          .strategy
          .fetch_quote::<HYUSD, XSOL>(amount, user, slippage_bps)
          .await,
      ),
      (XSOL::MINT, HYUSD::MINT) => (
        Operation::SwapLeverToStable,
        "Swap xSOL to hyUSD",
        self
          .strategy
          .fetch_quote::<XSOL, HYUSD>(amount, user, slippage_bps)
          .await,
      ),
      (HYUSD::MINT, SHYUSD::MINT) => (
        Operation::DepositToStabilityPool,
        "Deposit hyUSD to Stability Pool",
        self
          .strategy
          .fetch_quote::<HYUSD, SHYUSD>(amount, user, slippage_bps)
          .await,
      ),
      _ => Err(anyhow::anyhow!("Unsupported pair")),
    };

    let quote = quote_result?;
    let metadata = QuoteMetadata::new(operation, description);

    Ok((quote, metadata))
  }
}
