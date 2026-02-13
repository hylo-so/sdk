//! Quote metadata types

use hylo_core::stability_mode::StabilityMode;

/// Operation type for a quote
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
  MintStablecoin,
  RedeemStablecoin,
  MintLevercoin,
  RedeemLevercoin,
  SwapStableToLever,
  SwapLeverToStable,
  LstSwap,
  DepositToStabilityPool,
  WithdrawFromStabilityPool,
  WithdrawAndRedeemFromStabilityPool,
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
      Operation::LstSwap => "swap_lst",
      Operation::DepositToStabilityPool => "user_deposit",
      Operation::WithdrawFromStabilityPool => "user_withdraw",
      Operation::WithdrawAndRedeemFromStabilityPool => {
        "user_withdraw_and_redeem"
      }
    }
  }

  /// Whether this operation is quotable given stability mode and pool state.
  #[must_use]
  pub(crate) const fn quotable(
    self,
    mode: StabilityMode,
    pool_has_levercoin: bool,
  ) -> bool {
    use StabilityMode::{Depeg, Mode1, Mode2, Normal};

    let not_depegged = !matches!(mode, Depeg);
    let normal_or_mode1 = matches!(mode, Normal | Mode1);
    let deposit_allowed = matches!(mode, Normal | Mode1 | Mode2);

    match self {
      Self::MintStablecoin | Self::SwapLeverToStable => normal_or_mode1,

      Self::RedeemStablecoin | Self::LstSwap => true,

      Self::DepositToStabilityPool => deposit_allowed,

      Self::MintLevercoin | Self::RedeemLevercoin | Self::SwapStableToLever => {
        not_depegged
      }

      Self::WithdrawFromStabilityPool => !pool_has_levercoin,
      Self::WithdrawAndRedeemFromStabilityPool => {
        pool_has_levercoin && not_depegged
      }
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
