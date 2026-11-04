use anchor_lang::prelude::{
  borsh, AnchorDeserialize, AnchorSerialize, InitSpace, Pubkey,
};

use crate::error::CoreError;

const MIN_TTL_SECS: u64 = 2 * 60 * 60;
const MAX_TTL_SECS: u64 = 30 * 24 * 60 * 60;

/// Address selected for rotation.
#[derive(
  Debug,
  Clone,
  Copy,
  PartialEq,
  Eq,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
)]
pub enum AddressField {
  Admin,
  Treasury,
  PauseAuthority,
}

impl AsRef<[u8]> for AddressField {
  fn as_ref(&self) -> &[u8] {
    match self {
      AddressField::Admin => &[0],
      AddressField::Treasury => &[1],
      AddressField::PauseAuthority => &[2],
    }
  }
}

/// State containing addresses selected for rotation.
pub trait RotatableAddresses {
  /// Returns the address selected for rotation.
  ///
  /// # Errors
  ///
  /// * The selected address is not supported.
  fn get_address(
    &self,
    address_field: AddressField,
  ) -> Result<Pubkey, CoreError>;

  /// Replaces the address selected for rotation.
  ///
  /// # Errors
  ///
  /// * The selected address is not supported.
  fn set_address(
    &mut self,
    address_field: AddressField,
    address: Pubkey,
  ) -> Result<(), CoreError>;
}

/// Pending address rotation.
#[derive(
  Debug,
  Clone,
  Copy,
  PartialEq,
  Eq,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
)]
pub struct AddressUpdateProposalData {
  pub address_field: AddressField,
  pub new_address: Pubkey,
  pub proposal_time: i64,
  pub ttl_secs: u64,
  pub approved: bool,
}

/// Pending address rotation shared with program-owned accounts.
pub type AddressUpdateProposal = AddressUpdateProposalData;

impl AddressUpdateProposalData {
  /// Creates a proposal for an address change.
  ///
  /// # Errors
  ///
  /// * The proposed address is unchanged or the TTL is outside its bounds.
  pub fn new(
    address_field: AddressField,
    current_address: Pubkey,
    new_address: Pubkey,
    proposal_time: i64,
    ttl_secs: u64,
  ) -> Result<AddressUpdateProposal, CoreError> {
    (current_address != new_address)
      .then_some(())
      .ok_or(CoreError::AddressUpdateUnchangedAddress)?;
    (MIN_TTL_SECS..=MAX_TTL_SECS)
      .contains(&ttl_secs)
      .then_some(())
      .ok_or(CoreError::AddressUpdateInvalidTtl)?;
    Ok(AddressUpdateProposal {
      address_field,
      new_address,
      proposal_time,
      ttl_secs,
      approved: false,
    })
  }

  /// Checks the inclusive proposal lifetime.
  #[must_use]
  pub fn is_active(&self, clock_time: i64) -> bool {
    self
      .ttl_secs
      .try_into()
      .ok()
      .and_then(|ttl| self.proposal_time.checked_add(ttl))
      .is_some_and(|expires| {
        (self.proposal_time..=expires).contains(&clock_time)
      })
  }

  /// Approves an active proposal.
  ///
  /// # Errors
  ///
  /// * The proposal is inactive or already approved.
  pub fn approve(&mut self, clock_time: i64) -> Result<(), CoreError> {
    self
      .is_active(clock_time)
      .then_some(())
      .ok_or(CoreError::AddressUpdateExpired)?;
    (!self.approved)
      .then_some(())
      .ok_or(CoreError::AddressUpdateAlreadyApproved)?;
    self.approved = true;
    Ok(())
  }

  /// Checks whether the proposal can be accepted.
  ///
  /// # Errors
  ///
  /// * The proposal is inactive or has not been approved.
  pub fn validate_acceptance(&self, clock_time: i64) -> Result<(), CoreError> {
    self
      .is_active(clock_time)
      .then_some(())
      .ok_or(CoreError::AddressUpdateExpired)?;
    self
      .approved
      .then_some(())
      .ok_or(CoreError::AddressUpdateNotApproved)
  }
}
