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
  #[msg("Arithmetic error during LST to LST conversion.")]
  LstLstPriceConversion,
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
  // `fee_controller`
  #[msg("Over/underflow while computing fee extraction for transaction.")]
  FeeExtraction,
  #[msg("No valid mint fee for levercoin. Projected stability mode is Depeg.")]
  NoValidLevercoinMintFee,
  #[msg("No valid redeem fee for levercoin due to Depeg.")]
  NoValidLevercoinRedeemFee,
  #[msg("No valid mint fee for stablecoin due to Mode2 or Depeg.")]
  NoValidStablecoinMintFee,
  #[msg("No valid fee for swap due to Mode2 or Depeg.")]
  NoValidSwapFee,
  #[msg("Fees cannot exceed configured maximum.")]
  InvalidFees,
  // `exchange_context`
  #[msg("Arithmetic error or missing data while computing levercoin NAV.")]
  LevercoinNav,
  #[msg("Over/underflow while computing total SOL for destination fee.")]
  DestinationFeeSol,
  #[msg(
    "Over/underflow while computing total stablecoin for destination fee."
  )]
  DestinationFeeStablecoin,
  #[msg("There is no next stability threshold; current mode is Depeg.")]
  NoNextStabilityThreshold,
  #[msg("Requested amount of stablecoin over max mintable limit.")]
  RequestedStablecoinOverMaxMintable,
  // `stability_pool_math`
  #[msg("Arithmetic error while computing LP token NAV.")]
  LpTokenNav,
  #[msg("Arithmetic error while computing LP token amount to give to user.")]
  LpTokenOut,
  #[msg("Arithmetic error while computing amount of stablecoin to swap.")]
  StablecoinToSwap,
  #[msg("Arithmetic error while computing amount of token to withdraw.")]
  TokenWithdraw,
  // `yields`
  #[msg("Yield harvest configuration percentages failed validation.")]
  YieldHarvestConfigValidation,
  #[msg("Arithmetic error while computing yield harvest allocation.")]
  YieldHarvestAllocation,
  // `virtual_stablecoin`
  #[msg("Overflow while minting virtual stablecoin.")]
  MintOverflow,
  #[msg("Overflow while burning virtual stablecoin.")]
  BurnUnderflow,
  #[msg("Cannot mint zero amount of virtual stablecoin.")]
  MintZero,
  #[msg("Cannot burn zero amount of virtual stablecoin.")]
  BurnZero,
  // `interp`
  #[msg("Interpolation requires at least two points.")]
  InterpInsufficientPoints,
  #[msg("Interpolation points must have strictly increasing x-coordinates.")]
  InterpPointsNotMonotonic,
  #[msg("Interpolation input is outside the valid domain.")]
  InterpOutOfDomain,
  #[msg("Arithmetic overflow during interpolation calculation.")]
  InterpArithmetic,
  // `interpolated_fees`
  #[msg("Failed to convert collateral ratio from u64 to i64.")]
  CollateralRatioConversion,
  #[msg("Failed to convert interpolated fee from i64 to u64.")]
  InterpFeeConversion,
  // `funding_rate`
  #[msg("Funding rate configuration failed validation.")]
  FundingRateValidation,
  #[msg("Arithmetic error while applying funding rate.")]
  FundingRateApply,
  // `exo_exchange_context`
  #[msg("Arithmetic error converting exo collateral to protocol token.")]
  ExoToToken,
  #[msg("Arithmetic error converting protocol token to exo collateral.")]
  ExoFromToken,
  #[msg("Precision conversion failed for exo collateral price.")]
  ExoPriceConversion,
  #[msg("Over/underflow projecting exo collateral total for fee.")]
  ExoDestinationCollateral,
  #[msg("Over/underflow projecting stablecoin total for exo fee.")]
  ExoDestinationStablecoin,
}
