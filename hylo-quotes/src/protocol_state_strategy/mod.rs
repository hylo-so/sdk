mod exchange;
mod stability_pool;

use hylo_clients::protocol_state::StateProvider;

pub struct ProtocolStateStrategy<S: StateProvider> {
  pub(crate) state_provider: S,
}

impl<S: StateProvider> ProtocolStateStrategy<S> {
  #[must_use]
  pub fn new(state_provider: S) -> Self {
    Self { state_provider }
  }
}
