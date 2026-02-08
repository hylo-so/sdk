use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hylo_core::solana_clock::SolanaClock;
use hylo_core::stability_mode::StabilityMode;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::quote_metadata::{Operation, QuoteMetadata};
use crate::quote_strategy::QuoteStrategy;
use crate::ExecutableQuoteValue;

macro_rules! runtime_quote_strategies {
    ($(($in:ty, $out:ty, $op:expr, $desc:expr)),* $(,)?) => {
      /// Runtime dispatch trait bridging untyped `Pubkey` pair to typed `QuoteStrategy`.
      #[async_trait]
      pub trait RuntimeQuoteStrategy<C: SolanaClock>: $( QuoteStrategy<$in, $out, C> + )* {
        /// Fetches quote based on input and output mints.
        async fn runtime_quote(
          &self,
          input_mint: Pubkey,
          output_mint: Pubkey,
          amount_in: u64,
          user: Pubkey,
          slippage_tolerance: u64,
        ) -> Result<ExecutableQuoteValue> {
          match (input_mint, output_mint) {
            $(
              (<$in>::MINT, <$out>::MINT) => {
                let quote = QuoteStrategy::<$in, $out, C>::get_quote(self, amount_in, user, slippage_tolerance).await?;
                Ok(quote.into())
              },
            )*
            _ => Err(anyhow!("Unsupported pair")),
          }
        }

        /// Fetches quote based on input and output mints with relevant metadata.
        async fn runtime_quote_with_metadata(
          &self,
          input_mint: Pubkey,
          output_mint: Pubkey,
          amount_in: u64,
          user: Pubkey,
          slippage_tolerance: u64,
        ) -> Result<(ExecutableQuoteValue, QuoteMetadata)> {
          match (input_mint, output_mint) {
            $(
              (<$in>::MINT, <$out>::MINT) => {
                let quote = QuoteStrategy::<$in, $out, C>::get_quote(self, amount_in, user, slippage_tolerance).await?;
                Ok((quote.into(), QuoteMetadata::new($op, $desc)))
              },
            )*
            _ => Err(anyhow!("Unsupported pair")),
          }
        }
      }

      /// All quotable pairs; used by [`quotable_pairs_for_mode`].
      pub(crate) const ALL_QUOTABLE_PAIRS: &[(Pubkey, Pubkey, Operation, &'static str)] = &[
        $( (<$in>::MINT, <$out>::MINT, $op, $desc), )*
      ];
    };
}

runtime_quote_strategies! {
  (JITOSOL, HYUSD, Operation::MintStablecoin, "Mint hyUSD with JitoSOL"),
  (HYUSD, JITOSOL, Operation::RedeemStablecoin, "Redeem hyUSD for JitoSOL"),
  (HYLOSOL, HYUSD, Operation::MintStablecoin, "Mint hyUSD with hyloSOL"),
  (HYUSD, HYLOSOL, Operation::RedeemStablecoin, "Redeem hyUSD for hyloSOL"),
  (JITOSOL, XSOL, Operation::MintLevercoin, "Mint xSOL with JitoSOL"),
  (XSOL, JITOSOL, Operation::RedeemLevercoin, "Redeem xSOL for JitoSOL"),
  (HYLOSOL, XSOL, Operation::MintLevercoin, "Mint xSOL with hyloSOL"),
  (XSOL, HYLOSOL, Operation::RedeemLevercoin, "Redeem xSOL for hyloSOL"),
  (HYUSD, XSOL, Operation::SwapStableToLever, "Swap hyUSD to xSOL"),
  (XSOL, HYUSD, Operation::SwapLeverToStable, "Swap xSOL to hyUSD"),
  (JITOSOL, HYLOSOL, Operation::LstSwap, "Swap JitoSOL to hyloSOL"),
  (HYLOSOL, JITOSOL, Operation::LstSwap, "Swap hyloSOL to JitoSOL"),
  (HYUSD, SHYUSD, Operation::DepositToStabilityPool, "Deposit hyUSD to Stability Pool"),
  (SHYUSD, HYUSD, Operation::WithdrawFromStabilityPool, "Withdraw hyUSD from Stability Pool"),
  (SHYUSD, JITOSOL, Operation::WithdrawAndRedeemFromStabilityPool, "Withdraw sHYUSD and redeem for JitoSOL"),
  (SHYUSD, HYLOSOL, Operation::WithdrawAndRedeemFromStabilityPool, "Withdraw sHYUSD and redeem for hyloSOL"),
}

#[must_use]
pub(crate) const fn operation_allowed_in_mode(
  op: Operation,
  mode: StabilityMode,
) -> bool {
  use StabilityMode::{Depeg, Mode1, Mode2, Normal};

  let not_depegged = !matches!(mode, Depeg);
  let normal_or_mode1 = matches!(mode, Normal | Mode1);
  let deposit_allowed = matches!(mode, Normal | Mode1 | Mode2);

  match op {
    Operation::MintStablecoin | Operation::SwapLeverToStable => normal_or_mode1,

    Operation::RedeemStablecoin
    | Operation::LstSwap
    | Operation::WithdrawFromStabilityPool => true,

    Operation::DepositToStabilityPool => deposit_allowed,

    Operation::MintLevercoin
    | Operation::RedeemLevercoin
    | Operation::SwapStableToLever
    | Operation::WithdrawAndRedeemFromStabilityPool => not_depegged,
  }
}

/// Returns an iterator over supported quote pairs that are quotable in the
/// given stability mode.
///
/// Use this for mode-aware endpoints (e.g. public-api quotable-pairs filtered
/// by current protocol mode).
#[must_use = "iterator is lazy and does nothing unless consumed"]
pub fn quotable_pairs_for_mode(
  mode: StabilityMode,
) -> impl Iterator<Item = &'static (Pubkey, Pubkey, Operation, &'static str)> {
  ALL_QUOTABLE_PAIRS
    .iter()
    .filter(move |(_, _, op, _)| operation_allowed_in_mode(*op, mode))
}
