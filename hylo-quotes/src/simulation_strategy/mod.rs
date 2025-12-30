mod exchange;
mod stability_pool;

use hylo_clients::prelude::{ExchangeClient, StabilityPoolClient};

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
