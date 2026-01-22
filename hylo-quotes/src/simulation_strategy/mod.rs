//! Quote strategy using transaction simulation.
//!
//! Builds instructions and simulates transactions to extract output amounts
//! and compute units from emitted events.

mod exchange;
mod stability_pool;

use anchor_lang::prelude::Clock;
use async_trait::async_trait;
use hylo_clients::prelude::{ExchangeClient, StabilityPoolClient};
use hylo_clients::transaction::TransactionSyntax;

use crate::runtime_quote_strategy::RuntimeQuoteStrategy;

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

#[async_trait]
impl RuntimeQuoteStrategy<Clock> for SimulationStrategy {}

#[async_trait]
impl TransactionSyntax for SimulationStrategy {}
