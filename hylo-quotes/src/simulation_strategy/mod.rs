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
/// Returns `(compute_units, strategy)` where:
/// - If simulation returned `Some(cu)` with `cu > 0`: uses simulated value with
///   `Simulated` strategy
/// - If simulation returned `None` or `Some(0)`: falls back to estimated value
///   with `Estimated` strategy
///
/// This handles the case where simulation succeeds but doesn't provide compute
/// unit information.
pub(crate) fn extract_compute_units(
  compute_units: Option<u64>,
) -> (u64, ComputeUnitStrategy) {
  match compute_units {
    Some(cu) if cu > 0 => (cu, ComputeUnitStrategy::Simulated),
    Some(_) | None => (DEFAULT_CUS_WITH_BUFFER, ComputeUnitStrategy::Estimated),
  }
}
