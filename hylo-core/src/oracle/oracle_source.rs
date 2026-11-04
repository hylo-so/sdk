use anchor_lang::prelude::*;

/// Price source backing an observation.
#[derive(
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  Clone,
  Copy,
  Debug,
  PartialEq,
  Eq,
)]
pub enum OracleSource {
  PythLazer,
}
