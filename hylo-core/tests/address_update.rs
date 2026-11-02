use anchor_lang::prelude::Pubkey;
use hylo_core::address_update::{
  AddressField, AddressUpdateError, AddressUpdateProposal,
};

#[test]
fn proposal_enforces_ttl_bounds_and_rejects_unchanged_addresses() {
  let current = Pubkey::new_unique();
  let proposed = Pubkey::new_unique();
  let create = |ttl| {
    AddressUpdateProposal::new(AddressField::Admin, current, proposed, 100, ttl)
  };
  assert!(create(7_200).is_ok());
  assert!(create(30 * 24 * 60 * 60).is_ok());
  assert_eq!(create(7_199), Err(AddressUpdateError::InvalidTtl));
  assert_eq!(
    create(30 * 24 * 60 * 60 + 1),
    Err(AddressUpdateError::InvalidTtl)
  );
  assert_eq!(
    AddressUpdateProposal::new(
      AddressField::Admin,
      current,
      current,
      100,
      7_200
    ),
    Err(AddressUpdateError::UnchangedAddress)
  );
}

#[test]
fn approval_and_acceptance_check_the_inclusive_active_window() {
  let mut proposal = AddressUpdateProposal::new(
    AddressField::Admin,
    Pubkey::new_unique(),
    Pubkey::new_unique(),
    100,
    7_200,
  )
  .unwrap();
  assert_eq!(
    proposal.validate_acceptance(100),
    Err(AddressUpdateError::NotApproved)
  );
  assert_eq!(proposal.approve(99), Err(AddressUpdateError::Expired));
  assert!(proposal.approve(100).is_ok());
  assert_eq!(
    proposal.approve(100),
    Err(AddressUpdateError::AlreadyApproved)
  );
  assert!(proposal.validate_acceptance(7_300).is_ok());
  assert_eq!(
    proposal.validate_acceptance(7_301),
    Err(AddressUpdateError::Expired)
  );
}

#[test]
fn proposal_with_overflowing_expiry_is_inactive() {
  let proposal = AddressUpdateProposal::new(
    AddressField::Admin,
    Pubkey::new_unique(),
    Pubkey::new_unique(),
    i64::MAX,
    7_200,
  )
  .unwrap();
  assert!(!proposal.is_active(i64::MAX));
}

#[test]
fn proposal_serialization_preserves_existing_field_layout() {
  use anchor_lang::AnchorSerialize;

  let new_address = Pubkey::new_unique();
  let proposal = AddressUpdateProposal::new(
    AddressField::PauseAuthority,
    Pubkey::new_unique(),
    new_address,
    100,
    7_200,
  )
  .unwrap();
  let mut expected = vec![2];
  expected.extend_from_slice(new_address.as_ref());
  expected.extend_from_slice(&100_i64.to_le_bytes());
  expected.extend_from_slice(&7_200_u64.to_le_bytes());
  expected.push(0);
  assert_eq!(proposal.try_to_vec().unwrap(), expected);
  assert_eq!(AddressField::Admin.try_to_vec().unwrap(), [0]);
  assert_eq!(AddressField::Treasury.try_to_vec().unwrap(), [1]);
  assert_eq!(AddressField::PauseAuthority.try_to_vec().unwrap(), [2]);
}
