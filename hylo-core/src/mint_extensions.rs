use anchor_lang::prelude::Pubkey;
use anchor_spl::token_2022::spl_token_2022::extension::pausable::PausableConfig;
use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::{
  TransferFee, TransferFeeConfig as TransferFeeConfigExtension,
};
use anchor_spl::token_2022::spl_token_2022::extension::transfer_hook::TransferHook as TransferHookConfig;
use anchor_spl::token_2022::spl_token_2022::extension::ExtensionType::{
  ConfidentialMintBurn, ConfidentialTransferAccount,
  ConfidentialTransferFeeAmount, ConfidentialTransferFeeConfig,
  ConfidentialTransferMint, CpiGuard, DefaultAccountState, GroupMemberPointer,
  GroupPointer, ImmutableOwner, InterestBearingConfig, MemoTransfer,
  MetadataPointer, MintCloseAuthority, NonTransferable, NonTransferableAccount,
  Pausable, PausableAccount, PermanentDelegate, ScaledUiAmount, TokenGroup,
  TokenGroupMember, TokenMetadata, TransferFeeAmount, TransferFeeConfig,
  TransferHook, TransferHookAccount,
};
use anchor_spl::token_2022::spl_token_2022::extension::{
  BaseStateWithExtensions, ExtensionType, StateWithExtensions,
};
use anchor_spl::token_2022::spl_token_2022::state::Mint;
use anchor_spl::token_interface::spl_pod::optional_keys::OptionalNonZeroPubkey;

use crate::error::CoreError;
use crate::error::CoreError::{
  ArithmeticOverflow, CannotDeserializeMintExtension, MintExtensionBlacklisted,
  MintExtensionDisallowedConfig,
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
  /// Presence allowed but amounts must be adjusted per configuration.
  /// e.g.`ScaledUiAmount` and `InterestBearingConfig` stay on raw amount ×
  /// Pyth.
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
    TransferHook | Pausable => ExtensionClass::Guard,
    TransferFeeConfig | ScaledUiAmount | InterestBearingConfig => {
      ExtensionClass::Adjust
    }
    PermanentDelegate | DefaultAccountState => ExtensionClass::Policy,
    MintCloseAuthority
    | ConfidentialTransferMint
    | ConfidentialTransferFeeConfig
    | MetadataPointer
    | TokenMetadata
    | GroupPointer
    | TokenGroup
    | GroupMemberPointer
    | TokenGroupMember => ExtensionClass::Inert,
    _ => ExtensionClass::Reject,
  }
}

/// Class for a token-account extension against the collateral whitelist.
#[must_use]
pub const fn token_account_extension_class(
  ext: ExtensionType,
) -> ExtensionClass {
  match ext {
    TransferFeeAmount
    | TransferHookAccount
    | PausableAccount
    | NonTransferableAccount
    | ImmutableOwner => ExtensionClass::Inert,
    ConfidentialTransferAccount
    | ConfidentialTransferFeeAmount
    | MemoTransfer
    | CpiGuard => ExtensionClass::User,
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
    TransferHook => {
      let hook = mint.get_extension::<TransferHookConfig>()?;
      // check if hook program ID is not set
      if hook.program_id == OptionalNonZeroPubkey::default() {
        Ok(())
      } else {
        Err(MintExtensionDisallowedConfig)
      }
    }
    Pausable => {
      let pausable = mint.get_extension::<PausableConfig>()?;
      // check if mint is not paused
      if bool::from(pausable.paused) {
        Err(MintExtensionDisallowedConfig)
      } else {
        Ok(())
      }
    }
    _ => Err(MintExtensionBlacklisted),
  }
}

/// `TransferFeeConfig` when `owner` is Token-2022 and the extension is set.
#[must_use]
pub fn has_transfer_fee_extension(
  owner: &Pubkey,
  mint_data: &[u8],
) -> Option<TransferFeeConfigExtension> {
  if *owner == anchor_spl::token_2022::ID {
    StateWithExtensions::<Mint>::unpack(mint_data)
      .ok()
      .and_then(|mint| {
        mint
          .get_extension::<TransferFeeConfigExtension>()
          .ok()
          .copied()
      })
  } else {
    None
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
/// * Fee arithmetic overflow
pub fn calculate_transfer_fee_excluded_amount(
  owner: &Pubkey,
  mint_data: &[u8],
  transfer_fee_included_amount: u64,
  epoch: u64,
) -> Result<TransferFeeExcludedAmount, CoreError> {
  match epoch_transfer_fee(owner, mint_data, epoch) {
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
/// with the withheld amount on the transfer. See
/// <https://github.com/solana-labs/solana-program-library/pull/6874>.
///
/// # Errors
/// * Fee arithmetic overflow
pub fn calculate_transfer_fee_included_amount(
  owner: &Pubkey,
  mint_data: &[u8],
  transfer_fee_excluded_amount: u64,
  epoch: u64,
) -> Result<TransferFeeIncludedAmount, CoreError> {
  match epoch_transfer_fee(owner, mint_data, epoch) {
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
  owner: &Pubkey,
  mint_data: &[u8],
  epoch: u64,
) -> Option<TransferFee> {
  has_transfer_fee_extension(owner, mint_data)
    .map(|config| *config.get_epoch_fee(epoch))
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
    let data = mint_with!(TransferFeeConfigExtension, |_| {});
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn nonzero_transfer_fee_is_valid() {
    let data = mint_with!(TransferFeeConfigExtension, |config| {
      config.newer_transfer_fee.transfer_fee_basis_points = 100.into();
    });
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn transfer_fee_extension_requires_token_2022_owner() {
    let fee_mint = mint_with!(TransferFeeConfigExtension, |_| {});
    let hook_mint = mint_with!(TransferHookConfig, |_| {});
    assert!(
      has_transfer_fee_extension(&anchor_spl::token_2022::ID, &fee_mint)
        .is_some()
    );
    assert_eq!(
      has_transfer_fee_extension(&anchor_spl::token::ID, &fee_mint),
      None
    );
    assert_eq!(
      has_transfer_fee_extension(&anchor_spl::token_2022::ID, &hook_mint),
      None
    );
    assert_eq!(
      has_transfer_fee_extension(&anchor_spl::token_2022::ID, &classic_mint()),
      None
    );
    assert_eq!(
      has_transfer_fee_extension(&anchor_spl::token_2022::ID, &[0u8; 8]),
      None
    );
  }

  #[test]
  fn exact_in_includes_transfer_fee() {
    let data = mint_with!(TransferFeeConfigExtension, |config| {
      config.newer_transfer_fee.transfer_fee_basis_points = 100.into();
      config.newer_transfer_fee.maximum_fee = u64::MAX.into();
    });
    assert_eq!(
      calculate_transfer_fee_excluded_amount(
        &anchor_spl::token_2022::ID,
        &data,
        10_000,
        0,
      )
      .expect("excluded"),
      TransferFeeExcludedAmount {
        amount: 9_900,
        transfer_fee: 100,
      }
    );
    assert_eq!(
      calculate_transfer_fee_excluded_amount(
        &anchor_spl::token::ID,
        &classic_mint(),
        10_000,
        0,
      )
      .expect("classic"),
      TransferFeeExcludedAmount {
        amount: 10_000,
        transfer_fee: 0,
      }
    );
    assert_eq!(
      calculate_transfer_fee_included_amount(
        &anchor_spl::token_2022::ID,
        &data,
        9_900,
        0,
      )
      .expect("included"),
      TransferFeeIncludedAmount {
        amount: 10_000,
        transfer_fee: 100,
      }
    );
    assert_eq!(
      calculate_transfer_fee_included_amount(
        &anchor_spl::token_2022::ID,
        &data,
        0,
        0,
      )
      .expect("zero"),
      TransferFeeIncludedAmount {
        amount: 0,
        transfer_fee: 0,
      }
    );
  }

  #[test]
  fn exact_out_full_basis_points_uses_maximum_fee() {
    let data = mint_with!(TransferFeeConfigExtension, |config| {
      config.newer_transfer_fee.transfer_fee_basis_points = 10_000.into();
      config.newer_transfer_fee.maximum_fee = 50.into();
    });
    assert_eq!(
      calculate_transfer_fee_included_amount(
        &anchor_spl::token_2022::ID,
        &data,
        100,
        0,
      )
      .expect("max fee"),
      TransferFeeIncludedAmount {
        amount: 150,
        transfer_fee: 50,
      }
    );
  }

  #[test]
  fn unset_transfer_hook_is_valid() {
    let data = mint_with!(TransferHookConfig, |_| {});
    assert_eq!(validate_collateral_mint_extensions(&data), Ok(()));
  }

  #[test]
  fn set_transfer_hook_is_rejected() {
    let data = mint_with!(TransferHookConfig, |hook| {
      hook.program_id = Some(Pubkey::new_from_array([1; 32]))
        .try_into()
        .expect("pubkey");
    });
    assert_eq!(
      validate_collateral_mint_extensions(&data),
      Err(MintExtensionDisallowedConfig)
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
      Err(MintExtensionDisallowedConfig)
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
      mint_extension_class(NonTransferable),
      ExtensionClass::Reject
    );
    assert_eq!(
      mint_extension_class(ConfidentialMintBurn),
      ExtensionClass::Reject
    );
    assert_eq!(mint_extension_class(MemoTransfer), ExtensionClass::Reject);
    assert_eq!(
      mint_extension_class(TransferFeeConfig),
      ExtensionClass::Adjust
    );
    assert_eq!(
      mint_extension_class(InterestBearingConfig),
      ExtensionClass::Adjust
    );
    assert_eq!(mint_extension_class(ScaledUiAmount), ExtensionClass::Adjust);
    assert!(is_whitelisted_mint_extension(InterestBearingConfig));
    assert!(is_whitelisted_mint_extension(TransferFeeConfig));
    assert!(is_whitelisted_mint_extension(ScaledUiAmount));
  }

  #[test]
  fn token_account_classes() {
    assert_eq!(
      token_account_extension_class(ImmutableOwner),
      ExtensionClass::Inert
    );
    assert_eq!(
      token_account_extension_class(MemoTransfer),
      ExtensionClass::User
    );
    assert_eq!(
      token_account_extension_class(TransferFeeConfig),
      ExtensionClass::Reject
    );
  }
}
