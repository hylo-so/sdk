use anchor_spl::token_2022::spl_token_2022::extension::pausable::PausableConfig;
use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::{
  TransferFee, TransferFeeConfig,
};
use anchor_spl::token_2022::spl_token_2022::extension::transfer_hook::TransferHook;
use anchor_spl::token_2022::spl_token_2022::extension::{
  BaseStateWithExtensions, ExtensionType, StateWithExtensions,
};
use anchor_spl::token_2022::spl_token_2022::state::Mint;
use anchor_spl::token_interface::spl_pod::optional_keys::OptionalNonZeroPubkey;

use crate::error::CoreError;
use crate::error::CoreError::{
  ArithmeticOverflow, CannotDeserializeMintExtension, MintExtensionBlacklisted,
  MintExtensionConfigBlacklisted,
};

/// Policy class for a Token Extensions type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionClass {
  /// Presence fails registration and later collateral token CPIs.
  Reject,
  /// Presence allowed only while the extension config does not affect
  /// transfers or accounting. Enforced at registration and on every
  /// collateral token CPI.
  Guard,
  /// Presence allowed. Transfer-fee helpers adjust amounts;
  /// `ScaledUiAmount` and `InterestBearingConfig` stay on raw amount × Pyth.
  Adjust,
  /// Presence allowed. Listing decision only; no code path.
  Policy,
  /// Presence allowed. Transfer, mint, and burn ignore the extension.
  Inert,
  /// Account-level, owner opt-in. Can fail the owner's transaction only.
  User,
}

/// Class for a mint extension against the collateral whitelist.
#[must_use]
pub const fn mint_extension_class(ext: ExtensionType) -> ExtensionClass {
  match ext {
    ExtensionType::TransferHook | ExtensionType::Pausable => {
      ExtensionClass::Guard
    }
    ExtensionType::TransferFeeConfig
    | ExtensionType::ScaledUiAmount
    | ExtensionType::InterestBearingConfig => ExtensionClass::Adjust,
    ExtensionType::PermanentDelegate | ExtensionType::DefaultAccountState => {
      ExtensionClass::Policy
    }
    ExtensionType::MintCloseAuthority
    | ExtensionType::ConfidentialTransferMint
    | ExtensionType::ConfidentialTransferFeeConfig
    | ExtensionType::MetadataPointer
    | ExtensionType::TokenMetadata
    | ExtensionType::GroupPointer
    | ExtensionType::TokenGroup
    | ExtensionType::GroupMemberPointer
    | ExtensionType::TokenGroupMember => ExtensionClass::Inert,
    _ => ExtensionClass::Reject,
  }
}

/// Class for a token-account extension against the collateral whitelist.
#[must_use]
pub const fn token_account_extension_class(
  ext: ExtensionType,
) -> ExtensionClass {
  match ext {
    ExtensionType::TransferFeeAmount
    | ExtensionType::TransferHookAccount
    | ExtensionType::PausableAccount
    | ExtensionType::NonTransferableAccount
    | ExtensionType::ImmutableOwner => ExtensionClass::Inert,
    ExtensionType::ConfidentialTransferAccount
    | ExtensionType::ConfidentialTransferFeeAmount
    | ExtensionType::MemoTransfer
    | ExtensionType::CpiGuard => ExtensionClass::User,
    _ => ExtensionClass::Reject,
  }
}

/// Whether the mint extension may appear on collateral.
#[must_use]
pub const fn is_whitelisted_mint_extension(ext: ExtensionType) -> bool {
  !matches!(mint_extension_class(ext), ExtensionClass::Reject)
}

/// Rejects unlisted mint extensions and non-dormant Guard configs.
/// Call at listing and on every collateral token CPI.
///
/// # Errors
/// * Malformed mint TLV
/// * Unlisted extension present
/// * Guard extension not dormant
pub fn validate_collateral_mint_extensions(
  mint_data: &[u8],
) -> Result<(), CoreError> {
  let mint = StateWithExtensions::<Mint>::unpack(mint_data)
    .map_err(|_| CannotDeserializeMintExtension)?;
  mint
    .get_extension_types()
    .map_err(|_| CannotDeserializeMintExtension)?
    .into_iter()
    .try_for_each(|ext| validate_present_mint_extension(&mint, ext))
}

fn validate_present_mint_extension(
  mint: &StateWithExtensions<'_, Mint>,
  ext: ExtensionType,
) -> Result<(), CoreError> {
  match mint_extension_class(ext) {
    ExtensionClass::Reject => Err(MintExtensionBlacklisted),
    ExtensionClass::Guard => validate_extension_configuration(mint, ext),
    ExtensionClass::Adjust
    | ExtensionClass::Policy
    | ExtensionClass::Inert
    | ExtensionClass::User => Ok(()),
  }
}

fn validate_extension_configuration(
  mint: &StateWithExtensions<'_, Mint>,
  ext: ExtensionType,
) -> Result<(), CoreError> {
  match ext {
    ExtensionType::TransferHook => {
      let hook = mint
        .get_extension::<TransferHook>()
        .map_err(|_| CannotDeserializeMintExtension)?;
      // check if hook program ID is not set
      if hook.program_id == OptionalNonZeroPubkey::default() {
        Ok(())
      } else {
        Err(MintExtensionConfigBlacklisted)
      }
    }
    ExtensionType::Pausable => {
      let pausable = mint
        .get_extension::<PausableConfig>()
        .map_err(|_| CannotDeserializeMintExtension)?;
      // check if mint is not paused
      if bool::from(pausable.paused) {
        Err(MintExtensionConfigBlacklisted)
      } else {
        Ok(())
      }
    }
    _ => Err(MintExtensionBlacklisted),
  }
}

/// Gross transfer amount and the fee taken from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransferFeeIncludedAmount {
  pub amount: u64,
  pub transfer_fee: u64,
}

/// Net destination amount after the transfer fee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransferFeeExcludedAmount {
  pub amount: u64,
  pub transfer_fee: u64,
}

/// Exact-in: `transfer_fee_included_amount` is inclusive of the fee.
///
/// # Errors
/// * Malformed mint TLV
/// * Fee arithmetic overflow
pub fn calculate_transfer_fee_excluded_amount(
  mint_data: &[u8],
  transfer_fee_included_amount: u64,
  epoch: u64,
) -> Result<TransferFeeExcludedAmount, CoreError> {
  match epoch_transfer_fee(mint_data, epoch)? {
    Some(fee) => {
      let transfer_fee = fee
        .calculate_fee(transfer_fee_included_amount)
        .ok_or(ArithmeticOverflow)?;
      let amount = transfer_fee_included_amount
        .checked_sub(transfer_fee)
        .ok_or(ArithmeticOverflow)?;
      Ok(TransferFeeExcludedAmount {
        amount,
        transfer_fee,
      })
    }
    None => Ok(TransferFeeExcludedAmount {
      amount: transfer_fee_included_amount,
      transfer_fee: 0,
    }),
  }
}

/// Exact-out: `transfer_fee_excluded_amount` is the destination net.
///
/// Gross is SPL `calculate_pre_fee_amount` (ceiling), so destination
/// receives at least `net`. Fee is `calculate_fee` of that gross, matching
/// what Token-2022 withholds.
///
/// `calculate_inverse_fee` is not used. It is not a true inverse of
/// `calculate_fee` (`calculate_fee(x) >= inverse(x - calculate_fee(x))`).
/// Using that fee, or reconstructing gross as `net + inverse`, can disagree
/// with the withheld amount on the transfer. See: https://github.com/solana-labs/solana-program-library/pull/6874
///
/// # Errors
/// * Malformed mint TLV
/// * Fee arithmetic overflow
pub fn calculate_transfer_fee_included_amount(
  mint_data: &[u8],
  transfer_fee_excluded_amount: u64,
  epoch: u64,
) -> Result<TransferFeeIncludedAmount, CoreError> {
  match epoch_transfer_fee(mint_data, epoch)? {
    Some(fee) => {
      let amount = fee
        .calculate_pre_fee_amount(transfer_fee_excluded_amount)
        .ok_or(ArithmeticOverflow)?;
      let transfer_fee = fee.calculate_fee(amount).ok_or(ArithmeticOverflow)?;
      Ok(TransferFeeIncludedAmount {
        amount,
        transfer_fee,
      })
    }
    None => Ok(TransferFeeIncludedAmount {
      amount: transfer_fee_excluded_amount,
      transfer_fee: 0,
    }),
  }
}

fn epoch_transfer_fee(
  mint_data: &[u8],
  epoch: u64,
) -> Result<Option<TransferFee>, CoreError> {
  let mint = StateWithExtensions::<Mint>::unpack(mint_data)
    .map_err(|_| CannotDeserializeMintExtension)?;
  Ok(
    mint
      .get_extension::<TransferFeeConfig>()
      .ok()
      .map(|config| *config.get_epoch_fee(epoch)),
  )
}

#[cfg(test)]
mod tests {
  use anchor_lang::prelude::program_option::COption;
  use anchor_lang::prelude::Pubkey;
  use anchor_spl::token::spl_token::solana_program::program_pack::Pack;
  use anchor_spl::token::spl_token::state::Mint as SplMint;
  use anchor_spl::token_2022::spl_token_2022::extension::interest_bearing_mint::InterestBearingConfig;
  use anchor_spl::token_2022::spl_token_2022::extension::metadata_pointer::MetadataPointer;
  use anchor_spl::token_2022::spl_token_2022::extension::scaled_ui_amount::ScaledUiAmountConfig;
  use anchor_spl::token_2022::spl_token_2022::extension::{
    BaseStateWithExtensionsMut, Extension, PodStateWithExtensionsMut,
  };
  use anchor_spl::token_2022::spl_token_2022::pod::PodMint;

  use super::*;

  fn classic_mint() -> Vec<u8> {
    let mint = SplMint {
      mint_authority: COption::None,
      supply: 0,
      decimals: 8,
      is_initialized: true,
      freeze_authority: COption::None,
    };
    let mut data = vec![0u8; SplMint::LEN];
    SplMint::pack(mint, &mut data).expect("pack");
    data
  }

  macro_rules! mint_with {
    ($ty:ty, |$ext:pat_param| $body:block) => {{
      let mint_len = ExtensionType::try_calculate_account_len::<Mint>(&[
        <$ty as Extension>::TYPE,
      ])
      .expect("len");
      let mut data = vec![0u8; mint_len];
      {
        let mut mint =
          PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(&mut data)
            .expect("uninit");
        mint.base.is_initialized = true.into();
        mint.base.decimals = 8;
        {
          let $ext: &mut $ty = mint.init_extension::<$ty>(true).expect("ext");
          $body
        }
        mint.init_account_type().expect("account type");
      }
      data
    }};
  }

  #[test]
  fn classic_mint_is_valid() {
    assert_eq!(validate_collateral_mint_extensions(&classic_mint()), Ok(()));
  }

  #[test]
  fn malformed_mint_data_fails() {
    assert_eq!(
      validate_collateral_mint_extensions(&[0u8; 8]),
      Err(CannotDeserializeMintExtension)
    );
  }

  #[test]
  fn inert_metadata_pointer_is_valid() {
    let data = mint_with!(MetadataPointer, |_| {});
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn zero_transfer_fee_is_valid() {
    let data = mint_with!(TransferFeeConfig, |_| {});
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn nonzero_transfer_fee_is_valid() {
    let data = mint_with!(TransferFeeConfig, |config| {
      config.newer_transfer_fee.transfer_fee_basis_points = 100.into();
    });
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn exact_in_includes_transfer_fee() {
    let data = mint_with!(TransferFeeConfig, |config| {
      config.newer_transfer_fee.transfer_fee_basis_points = 100.into();
      config.newer_transfer_fee.maximum_fee = u64::MAX.into();
    });
    assert_eq!(
      calculate_transfer_fee_excluded_amount(&data, 10_000, 0)
        .expect("excluded"),
      TransferFeeExcludedAmount {
        amount: 9_900,
        transfer_fee: 100,
      }
    );
    assert_eq!(
      calculate_transfer_fee_excluded_amount(&classic_mint(), 10_000, 0)
        .expect("classic"),
      TransferFeeExcludedAmount {
        amount: 10_000,
        transfer_fee: 0,
      }
    );
    assert_eq!(
      calculate_transfer_fee_included_amount(&data, 9_900, 0)
        .expect("included"),
      TransferFeeIncludedAmount {
        amount: 10_000,
        transfer_fee: 100,
      }
    );
    assert_eq!(
      calculate_transfer_fee_included_amount(&data, 0, 0).expect("zero"),
      TransferFeeIncludedAmount {
        amount: 0,
        transfer_fee: 0,
      }
    );
  }

  #[test]
  fn exact_out_full_basis_points_uses_maximum_fee() {
    let data = mint_with!(TransferFeeConfig, |config| {
      config.newer_transfer_fee.transfer_fee_basis_points = 10_000.into();
      config.newer_transfer_fee.maximum_fee = 50.into();
    });
    assert_eq!(
      calculate_transfer_fee_included_amount(&data, 100, 0).expect("max fee"),
      TransferFeeIncludedAmount {
        amount: 150,
        transfer_fee: 50,
      }
    );
  }

  #[test]
  fn unset_transfer_hook_is_valid() {
    let data = mint_with!(TransferHook, |_| {});
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn set_transfer_hook_is_rejected() {
    let data = mint_with!(TransferHook, |hook| {
      hook.program_id = Some(Pubkey::new_from_array([1; 32]))
        .try_into()
        .expect("pubkey");
    });
    assert_eq!(
      validate_collateral_mint_extensions(&data),
      Err(MintExtensionConfigBlacklisted)
    );
  }

  #[test]
  fn unpaused_mint_is_valid() {
    let data = mint_with!(PausableConfig, |_| {});
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn paused_mint_is_rejected() {
    let data = mint_with!(PausableConfig, |pausable| {
      pausable.paused = true.into();
    });
    assert_eq!(
      validate_collateral_mint_extensions(&data),
      Err(MintExtensionConfigBlacklisted)
    );
  }

  #[test]
  fn interest_bearing_is_valid() {
    let data = mint_with!(InterestBearingConfig, |_| {});
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn scaled_ui_amount_is_valid() {
    let data = mint_with!(ScaledUiAmountConfig, |_| {});
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn unlisted_and_reject_classes() {
    assert_eq!(
      mint_extension_class(ExtensionType::NonTransferable),
      ExtensionClass::Reject
    );
    assert_eq!(
      mint_extension_class(ExtensionType::ConfidentialMintBurn),
      ExtensionClass::Reject
    );
    assert_eq!(
      mint_extension_class(ExtensionType::MemoTransfer),
      ExtensionClass::Reject
    );
    assert_eq!(
      mint_extension_class(ExtensionType::TransferFeeConfig),
      ExtensionClass::Adjust
    );
    assert_eq!(
      mint_extension_class(ExtensionType::InterestBearingConfig),
      ExtensionClass::Adjust
    );
    assert_eq!(
      mint_extension_class(ExtensionType::ScaledUiAmount),
      ExtensionClass::Adjust
    );
    assert!(is_whitelisted_mint_extension(
      ExtensionType::InterestBearingConfig
    ));
    assert!(is_whitelisted_mint_extension(
      ExtensionType::TransferFeeConfig
    ));
    assert!(is_whitelisted_mint_extension(ExtensionType::ScaledUiAmount));
  }

  #[test]
  fn token_account_classes() {
    assert_eq!(
      token_account_extension_class(ExtensionType::ImmutableOwner),
      ExtensionClass::Inert
    );
    assert_eq!(
      token_account_extension_class(ExtensionType::MemoTransfer),
      ExtensionClass::User
    );
    assert_eq!(
      token_account_extension_class(ExtensionType::TransferFeeConfig),
      ExtensionClass::Reject
    );
  }
}
