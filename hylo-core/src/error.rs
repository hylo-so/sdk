use anchor_lang::prelude::error_code;

#[error_code]
pub enum CoreError {
  // `lst::total_sol_cache`
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
  // `lst::sol_price`
  #[msg("Underflow in delta between current and previous LST prices.")]
  LstSolPriceDelta,
  #[msg("LstSolPrice delta failed due to non-adjacent epochs.")]
  LstSolPriceEpochOrder,
  #[msg("Cached LstSolPrice is not from current epoch.")]
  LstSolPriceOutdated,
  #[msg("Overflow while computing LstSolPrice conversion.")]
  LstSolPriceConversion,
  #[msg("Overflow while computing SolLstPrice conversion.")]
  SolLstPriceConversion,
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
  // `conversion`
  #[msg("Arithmetic error in conversion from levercoin to stablecoin.")]
  LeverToStable,
  #[msg("Arithmetic error in conversion from stablecoin to levercoin.")]
  StableToLever,
  #[msg("Arithmetic error in conversion from LST to protocol token.")]
  LstToToken,
  #[msg("Arithmetic error in conversion from protocol token to LST.")]
  TokenToLst,
  // `fees::controller`
  #[msg("Over/underflow while computing fee extraction for transaction.")]
  FeeExtraction,
  #[msg("No valid mint fee for levercoin. Projected rebalance mode is Depeg.")]
  NoValidLevercoinMintFee,
  #[msg("No valid redeem fee for levercoin due to Depeg.")]
  NoValidLevercoinRedeemFee,
  #[msg("No valid mint fee for stablecoin due to SellZone2 or Depeg.")]
  NoValidStablecoinMintFee,
  #[msg("No valid fee for swap due to SellZone2 or Depeg.")]
  NoValidSwapFee,
  #[msg("Fees cannot exceed configured maximum.")]
  InvalidFees,
  // `exchange_context`
  #[msg("Arithmetic error while computing levercoin NAV.")]
  LevercoinNav,
  #[msg("Levercoin supply not set on exchange context.")]
  LevercoinSupplyNotSet,
  #[msg("Over/underflow projecting total collateral.")]
  DestinationCollateral,
  #[msg("Over/underflow projecting total stablecoin.")]
  DestinationStablecoin,
  #[msg("Requested amount of stablecoin over max mintable limit.")]
  RequestedStablecoinOverMaxMintable,
  // `earn_pool_math`
  #[msg("Arithmetic error while computing LP token NAV.")]
  LpTokenNav,
  #[msg("Arithmetic error while computing LP token amount to give to user.")]
  LpTokenOut,
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
  // `interp`
  #[msg("Interpolation requires at least two points.")]
  InterpInsufficientPoints,
  #[msg("Interpolation points must have strictly increasing x-coordinates.")]
  InterpPointsNotMonotonic,
  #[msg("Interpolation input is outside the valid domain.")]
  InterpOutOfDomain,
  #[msg("Arithmetic overflow during interpolation calculation.")]
  InterpArithmetic,
  // `fees::curve_controller`
  #[msg("Failed to convert collateral ratio from u64 to i64.")]
  CollateralRatioConversion,
  #[msg("Failed to convert interpolated fee from i64 to u64.")]
  InterpFeeConversion,
  // `borrow_rate`
  #[msg("Borrow rate configuration failed validation.")]
  BorrowRateValidation,
  #[msg("Arithmetic error while applying borrow rate.")]
  BorrowRateApply,
  // `exo_exchange_context`
  #[msg("Arithmetic error converting exo collateral to protocol token.")]
  ExoToToken,
  #[msg("Arithmetic error converting protocol token to exo collateral.")]
  ExoFromToken,
  // `normalize_mint_exp`
  #[msg("Precision conversion failed while normalizing exo amount to N9.")]
  ExoAmountNormalization,
  #[msg("Arithmetic error converting exo collateral to USDC.")]
  ExoCollateralToUsdc,
  #[msg("Arithmetic error converting USDC to exo collateral.")]
  ExoUsdcToCollateral,
  #[msg("Arithmetic error converting LST to USDC via SOL.")]
  LstToUsdc,
  #[msg("Arithmetic error converting USDC to LST via SOL.")]
  UsdcToLst,
  // `rebalance::pricing`
  #[msg("Rebalancing deviation tolerance not within valid range.")]
  RebalanceDeviationValidation,
  #[msg(
    "Rebalance curve config multiplier is zero or has incorrect precision."
  )]
  RebalanceCurveConfigValidation,
  #[msg("Arithmetic error constructing rebalance price curve from oracle.")]
  RebalancePriceConstruction,
  #[msg("CR or price conversion failed in rebalance price curve.")]
  RebalancePriceConversion,
  #[msg("CR is outside the rebalance pricing curve domain.")]
  RebalanceOutOfDomain,
  #[msg("Rebalance amount projects CR outside the pricing curve domain.")]
  RebalanceAmountExceeded,
  #[msg("Rebalance price exceeds deviation tolerance from oracle spot.")]
  RebalanceDeviationExceeded,
  #[msg("Arithmetic error computing deviation tolerance band.")]
  RebalanceDeviationArithmetic,
  // `rebalance::math`
  #[msg("Arithmetic error while computing sell side liquidity.")]
  RebalanceSellSideLiquidity,
  #[msg("Arithmetic error while computing buy side target.")]
  RebalanceBuySideTarget,
  // `rebalance::pnl`
  #[msg("Arithmetic error while updating rebalance PnL cache.")]
  RebalancePnlCacheUpdate,
  #[msg("Arithmetic error while computing new rebalance PnL.")]
  RebalancePnlCacheNet,
  // `lst::stake_pool`
  #[msg("Division by zero computing SPL stake pool price.")]
  StakePoolDivByZero,
  // `oracle_config`
  #[msg("Oracle interval not in valid range.")]
  OracleIntervalSecsInvalid,
  #[msg("Oracle confidence tolerance not in valid range.")]
  OracleConfToleranceInvalid,
  // `rebalance::mode`
  #[msg("Range boundary did not match the expected bound variant.")]
  RangeUnexpectedBound,
  #[msg("Stablecoin mint threshold not in Neutral rebalance range.")]
  StablecoinMintThresholdInvalid,
  // `levercoin_limiter`
  #[msg("Levercoin market cap limit not in valid configuration range.")]
  LevercoinMarketCapLimitInvalid,
  #[msg("Cannot mint new levercoin as market cap limit has been reached.")]
  LevercoinMarketCapLimitReached,
  #[msg("Arithmetic error while computing levercoin market cap limit.")]
  LevercoinMarketCapArithmetic,
}
