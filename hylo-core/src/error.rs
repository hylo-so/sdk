use anchor_lang::prelude::*;

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
}
