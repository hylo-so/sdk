//! Error types for offchain stats computation.

use anchor_lang::prelude::Pubkey;
use thiserror::Error;

/// Errors from earn pool yield statistics.
#[derive(Debug, Error)]
pub enum StatsError {
  #[error("Arithmetic error computing epoch yield rate.")]
  EpochYieldRate,
  #[error("Arithmetic error computing LST epoch growth.")]
  LstEpochGrowth,
  #[error("Arithmetic error computing projected pool inflow.")]
  ProjectedInflow,
  #[error("Overflow summing realized yield.")]
  RealizedYieldOverflow,
  #[error("Overflow summing projected LST inflow.")]
  ProjectedLstInflowOverflow,
  #[error("Overflow summing projected exo inflow.")]
  ProjectedExoInflowOverflow,
  #[error("Overflow summing gross projected inflow.")]
  ProjectedInflowOverflow,
  #[error("Overflow summing outstanding pool drawdown.")]
  PoolDrawdownOverflow,
  #[error("Overflow computing LST vault SOL value.")]
  LstVaultValueOverflow,
  #[error("No previous epoch to measure.")]
  NoPreviousEpoch,
  #[error("Non-positive epoch duration.")]
  NonPositiveEpochDuration,
  #[error("No block found at or after slot {0}.")]
  NoBlockAtOrAfterSlot(u64),
  #[error("Missing stats accounts: {0:?}")]
  MissingAccounts(Vec<Pubkey>),
  #[error("Expected {expected} stats accounts, got {actual}.")]
  AccountCountMismatch { expected: usize, actual: usize },
  #[error("Failed to deserialize clock sysvar: {0}")]
  ClockDeserialize(#[from] bincode::Error),
}
