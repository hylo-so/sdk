//! Quote provider with mint matching logic

use anchor_client::solana_sdk::clock::Clock;
use anchor_lang::prelude::Pubkey;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::quote_metadata::{Operation, QuoteMetadata};
use crate::quote_strategy::QuoteStrategy;
use crate::Quote;

/// Provider that matches mint pairs and fetches quotes
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
    + QuoteStrategy<HYUSD, SHYUSD, Clock>,
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
    amount: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> anyhow::Result<(Quote, QuoteMetadata)> {
    let (operation, description, quote_result) = match (input_mint, output_mint)
    {
      (JITOSOL::MINT, HYUSD::MINT) => (
        Operation::MintStablecoin,
        "Mint hyUSD with JitoSOL",
        <S as QuoteStrategy<JITOSOL, HYUSD, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYUSD::MINT, JITOSOL::MINT) => (
        Operation::RedeemStablecoin,
        "Redeem hyUSD for JitoSOL",
        <S as QuoteStrategy<HYUSD, JITOSOL, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYLOSOL::MINT, HYUSD::MINT) => (
        Operation::MintStablecoin,
        "Mint hyUSD with hyloSOL",
        <S as QuoteStrategy<HYLOSOL, HYUSD, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYUSD::MINT, HYLOSOL::MINT) => (
        Operation::RedeemStablecoin,
        "Redeem hyUSD for hyloSOL",
        <S as QuoteStrategy<HYUSD, HYLOSOL, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (JITOSOL::MINT, XSOL::MINT) => (
        Operation::MintLevercoin,
        "Mint xSOL with JitoSOL",
        <S as QuoteStrategy<JITOSOL, XSOL, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (XSOL::MINT, JITOSOL::MINT) => (
        Operation::RedeemLevercoin,
        "Redeem xSOL for JitoSOL",
        <S as QuoteStrategy<XSOL, JITOSOL, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYLOSOL::MINT, XSOL::MINT) => (
        Operation::MintLevercoin,
        "Mint xSOL with hyloSOL",
        <S as QuoteStrategy<HYLOSOL, XSOL, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (XSOL::MINT, HYLOSOL::MINT) => (
        Operation::RedeemLevercoin,
        "Redeem xSOL for hyloSOL",
        <S as QuoteStrategy<XSOL, HYLOSOL, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYUSD::MINT, XSOL::MINT) => (
        Operation::SwapStableToLever,
        "Swap hyUSD to xSOL",
        <S as QuoteStrategy<HYUSD, XSOL, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (XSOL::MINT, HYUSD::MINT) => (
        Operation::SwapLeverToStable,
        "Swap xSOL to hyUSD",
        <S as QuoteStrategy<XSOL, HYUSD, Clock>>::get_quote(
          &self.strategy,
          amount,
          user,
          slippage_tolerance,
        )
        .await,
      ),
      (HYUSD::MINT, SHYUSD::MINT) => (
        Operation::DepositToStabilityPool,
        "Deposit hyUSD to Stability Pool",
        <S as QuoteStrategy<HYUSD, SHYUSD, Clock>>::get_quote(
          &self.strategy,
          amount,
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
