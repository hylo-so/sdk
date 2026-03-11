//! Quote metadata types

/// Operation type for a quote
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
  MintStablecoinLst,
  RedeemStablecoinLst,
  MintLevercoinLst,
  RedeemLevercoinLst,
  ConvertStableToLeverLst,
  ConvertLeverToStableLst,
  SwapLstToLst,
  DepositToStabilityPool,
  WithdrawFromStabilityPool,
}

impl Operation {
  #[must_use]
  pub const fn as_str(&self) -> &'static str {
    match self {
      Operation::MintStablecoinLst => "mint_stablecoin_lst",
      Operation::RedeemStablecoinLst => "redeem_stablecoin_lst",
      Operation::MintLevercoinLst => "mint_levercoin_lst",
      Operation::RedeemLevercoinLst => "redeem_levercoin_lst",
      Operation::ConvertStableToLeverLst => "convert_stable_to_lever_lst",
      Operation::ConvertLeverToStableLst => "convert_lever_to_stable_lst",
      Operation::SwapLstToLst => "swap_lst_to_lst",
      Operation::DepositToStabilityPool => "user_deposit",
      Operation::WithdrawFromStabilityPool => "user_withdraw",
    }
  }
}

impl AsRef<str> for Operation {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl std::fmt::Display for Operation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

/// Metadata for a quote route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteMetadata {
  /// The operation this quote represents (useful for metrics)
  pub operation: Operation,

  /// Human-readable route description with operation details (eg, which LST)
  pub description: String,
}

impl QuoteMetadata {
  #[must_use]
  pub fn new(operation: Operation, description: impl Into<String>) -> Self {
    Self {
      operation,
      description: description.into(),
    }
  }
}
