use anchor_lang::prelude::error_code;

#[error_code]
pub enum CoreError {
  // `total_sol_cache`
  #[msg("Cannot decrement TotalSolCache due to outdated epoch.")]
  TotalSolCacheDecrement = 7000,
  #[msg("Cannot increment TotalSolCache due to outdated epoch.")]
  TotalSolCacheIncrement,
  #[msg("Increment overflow in TotalSolCache.")]
  TotalSolCacheOverflow,
  #[msg("Decrement underflow in TotalSolCache.")]
  TotalSolCacheUnderflow,
  #[msg("TotalSolCache is not valid for the current epoch.")]
  TotalSolCacheOutdated,
  // `lst_sol_price`
  #[msg("Underflow in delta between current and previous LST prices.")]
  LstSolPriceDelta,
  #[msg("LstSolPrice delta failed due to non-adjacent epochs.")]
  LstSolPriceEpochOrder,
  #[msg("Cached LstSolPrice is not from current epoch.")]
  LstSolPriceOutdated,
  #[msg("Overflow while computing LstSolPrice conversion.")]
  LstSolPriceConversion,
  // `pyth`
  #[msg("Oracle confidence interval is too wide.")]
  PythOracleConfidence,
  #[msg("Oracle exponent is out of range.")]
  PythOracleExponent,
  #[msg("Oracle yielded a negative price which can't be unsigned.")]
  PythOracleNegativePrice,
  #[msg("Oracle time is negative.")]
  PythOracleNegativeTime,
  #[msg("Oracle did not yield a price within the configured age window.")]
  PythOracleOutdated,
  #[msg("Oracle price is out of range.")]
  PythOraclePriceRange,
  #[msg("Oracle publish slot greater than current slot.")]
  PythOracleSlotInvalid,
  #[msg("Oracle price update is not fully verified.")]
  PythOracleVerificationLevel,
  // `nav`
  #[msg("Overflow while computing collateral ratio.")]
  CollateralRatio,
  #[msg("Arithmetic error while computing max mintable stablecoin.")]
  MaxMintable,
  #[msg("Arithmetic error while computing max swappable stablecoin.")]
  MaxSwappable,
  #[msg("Arithmetic error while computing stability pool capitalization.")]
  StabilityPoolCap,
  #[msg("Arithmetic error while computing depegged stablecoin NAV.")]
  StablecoinNav,
  #[msg("Unable to compute max mintable stablecoin with target CR < 1.")]
  TargetCollateralRatioTooLow,
  #[msg("Overflow while computing total value locked in USD.")]
  TotalValueLocked,
  // `slippage_config`
  #[msg("Over/underflow while computing acceptable token amount.")]
  SlippageArithmetic,
  #[msg("Token output amount exceeds provided slippage configuration.")]
  SlippageExceeded,
  // `stability_mode`
  #[msg("Stability modes failed validation.")]
  StabilityValidation,
  // `conversion`
  #[msg("Arithmetic error in conversion from levercoin to stablecoin.")]
  LeverToStable,
  #[msg("Arithmetic error in conversion from stablecoin to levercoin.")]
  StableToLever,
  #[msg("Arithmetic error in conversion from LST to protocol token.")]
  LstToToken,
  #[msg("Arithmetic error in conversion from protocol token to LST.")]
  TokenToLst,
}
