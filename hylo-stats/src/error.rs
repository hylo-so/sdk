//! Error types for offchain stats computation.

use thiserror::Error;

/// Errors from earn pool yield statistics math.
#[derive(Debug, Clone, Copy, Error)]
pub enum StatsError {
  #[error("Arithmetic error computing epoch yield rate.")]
  EpochYieldRate,
  #[error("Arithmetic error computing LST epoch growth.")]
  LstEpochGrowth,
  #[error("Arithmetic error computing projected pool inflow.")]
  ProjectedInflow,
}
