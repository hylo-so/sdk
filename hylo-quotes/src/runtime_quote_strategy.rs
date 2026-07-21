use anchor_lang::prelude::Pubkey;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{
  TokenMint, CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC, XSOL,
};

use crate::protocol_state::ProtocolState;
use crate::quote_metadata::{Operation, QuoteMetadata};
use crate::quote_strategy::QuoteStrategy;
use crate::token_operation::TokenOperation;
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

      impl<C: SolanaClock> ProtocolState<C> {
        /// Largest executable input for the pair in input-mint atoms.
        ///
        /// # Errors
        /// * Unsupported pair or route gated in current state
        pub fn runtime_max_input(
          &self,
          input_mint: Pubkey,
          output_mint: Pubkey,
        ) -> Result<u64> {
          match (input_mint, output_mint) {
            $(
              (<$in>::MINT, <$out>::MINT) => {
                Ok(TokenOperation::<$in, $out>::max_input(self)?.bits)
              },
            )*
            _ => Err(anyhow!("Unsupported pair")),
          }
        }
      }
    };
}

runtime_quote_strategies! {
  (JITOSOL, HYUSD, Operation::MintStablecoinLst, "Mint hyUSD with JitoSOL"),
  (HYUSD, JITOSOL, Operation::RedeemStablecoinLst, "Redeem hyUSD for JitoSOL"),
  (HYLOSOL, HYUSD, Operation::MintStablecoinLst, "Mint hyUSD with hyloSOL"),
  (HYUSD, HYLOSOL, Operation::RedeemStablecoinLst, "Redeem hyUSD for hyloSOL"),
  (JITOSOL, XSOL, Operation::MintLevercoinLst, "Mint xSOL with JitoSOL"),
  (XSOL, JITOSOL, Operation::RedeemLevercoinLst, "Redeem xSOL for JitoSOL"),
  (HYLOSOL, XSOL, Operation::MintLevercoinLst, "Mint xSOL with hyloSOL"),
  (XSOL, HYLOSOL, Operation::RedeemLevercoinLst, "Redeem xSOL for hyloSOL"),
  (HYUSD, XSOL, Operation::ConvertStableToLeverLst, "Convert hyUSD to xSOL"),
  (XSOL, HYUSD, Operation::ConvertLeverToStableLst, "Convert xSOL to hyUSD"),
  (JITOSOL, HYLOSOL, Operation::SwapLstToLst, "Swap JitoSOL to hyloSOL"),
  (HYLOSOL, JITOSOL, Operation::SwapLstToLst, "Swap hyloSOL to JitoSOL"),
  (JITOSOL, USDC, Operation::SwapLstToUsdc, "Swap JitoSOL for USDC"),
  (HYLOSOL, USDC, Operation::SwapLstToUsdc, "Swap hyloSOL for USDC"),
  (USDC, JITOSOL, Operation::SwapUsdcToLst, "Swap USDC for JitoSOL"),
  (USDC, HYLOSOL, Operation::SwapUsdcToLst, "Swap USDC for hyloSOL"),
  (CBBTC, USDC, Operation::SwapExoToUsdc, "Swap cbBTC for USDC"),
  (USDC, CBBTC, Operation::SwapUsdcToExo, "Swap USDC for cbBTC"),
  (HYUSD, SHYUSD, Operation::DepositToEarnPool, "Deposit hyUSD to Earn Pool"),
  (SHYUSD, HYUSD, Operation::WithdrawFromEarnPool, "Withdraw hyUSD from Earn Pool"),
  (USDC, HYUSD, Operation::MintStablecoinUsdc, "Mint hyUSD with USDC"),
  (HYUSD, USDC, Operation::RedeemStablecoinUsdc, "Redeem hyUSD for USDC"),
  (CBBTC, HYUSD, Operation::MintStablecoinExo, "Mint hyUSD with cbBTC"),
  (HYUSD, CBBTC, Operation::RedeemStablecoinExo, "Redeem hyUSD for cbBTC"),
  (CBBTC, XBTC, Operation::MintLevercoinExo, "Mint xBTC with cbBTC"),
  (XBTC, CBBTC, Operation::RedeemLevercoinExo, "Redeem xBTC for cbBTC"),
  (HYUSD, XBTC, Operation::ConvertStableToLeverExo, "Convert hyUSD to xBTC"),
  (XBTC, HYUSD, Operation::ConvertLeverToStableExo, "Convert xBTC to hyUSD"),
}
