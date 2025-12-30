mod exchange;
mod stability_pool;

use hylo_clients::protocol_state::StateProvider;

// TODO(Levi): Get estimated compute units from simulation for each operation
// (see other quotes branch)
const ESTIMATED_COMPUTE_UNITS: u64 = 100_000;

pub struct ProtocolStateStrategy<S: StateProvider> {
  pub(crate) state_provider: S,
}

impl<S: StateProvider> ProtocolStateStrategy<S> {
  #[must_use]
  pub fn new(state_provider: S) -> Self {
    Self { state_provider }
  }
}
