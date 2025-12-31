//! Quote metadata types

/// Operation type for a quote
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
  MintStablecoin,
  RedeemStablecoin,
  MintLevercoin,
  RedeemLevercoin,
  SwapStableToLever,
  SwapLeverToStable,
  DepositToStabilityPool,
  WithdrawFromStabilityPool,
}

impl Operation {
  #[must_use]
  pub const fn as_str(&self) -> &'static str {
    match self {
      Operation::MintStablecoin => "mint_stablecoin",
      Operation::RedeemStablecoin => "redeem_stablecoin",
      Operation::MintLevercoin => "mint_levercoin",
      Operation::RedeemLevercoin => "redeem_levercoin",
      Operation::SwapStableToLever => "swap_stable_to_lever",
      Operation::SwapLeverToStable => "swap_lever_to_stable",
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
