use anchor_lang::prelude::*;

#[derive(
  Debug,
  Clone,
  Copy,
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  PartialEq,
  Eq,
)]
/// Observation indexes and an optional reference index.
pub struct OracleObservationConfig {
  pub index: u8,
  pub reference_index: Option<u8>,
}
