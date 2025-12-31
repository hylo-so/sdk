//! Quote strategy using transaction simulation.
//!
//! Builds instructions and simulates transactions to extract output amounts
//! and compute units from emitted events.

mod exchange;
mod stability_pool;

use hylo_clients::prelude::{ExchangeClient, StabilityPoolClient};

use crate::{ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER};

pub struct SimulationStrategy {
  pub(crate) exchange_client: ExchangeClient,
  pub(crate) stability_pool_client: StabilityPoolClient,
}

impl SimulationStrategy {
  #[must_use]
  pub fn new(
    exchange_client: ExchangeClient,
    stability_pool_client: StabilityPoolClient,
  ) -> Self {
    Self {
      exchange_client,
      stability_pool_client,
    }
  }
}

/// Extract compute units and strategy from simulation result.
///
/// Returns `(compute_units, strategy)`. If simulation provides compute units,
/// uses `Simulated` strategy; otherwise falls back to `Estimated` with default
/// buffered value.
pub(crate) fn resolve_compute_units(
  compute_units: Option<u64>,
) -> (u64, ComputeUnitStrategy) {
  match compute_units {
    Some(cu) if cu > 0 => (cu, ComputeUnitStrategy::Simulated),
    Some(_) | None => (DEFAULT_CUS_WITH_BUFFER, ComputeUnitStrategy::Estimated),
  }
}
