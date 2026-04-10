//! Quote strategy using transaction simulation.
//!
//! Builds instructions and simulates transactions to extract output
//! amounts and compute units from return data.

mod router;

use anchor_lang::prelude::Clock;
use async_trait::async_trait;
use hylo_clients::router_client::RouterClient;
use hylo_clients::transaction::TransactionSyntax;

use crate::runtime_quote_strategy::RuntimeQuoteStrategy;

pub struct SimulationStrategy {
  pub router_client: RouterClient,
}

impl SimulationStrategy {
  #[must_use]
  pub fn new(router_client: RouterClient) -> SimulationStrategy {
    SimulationStrategy { router_client }
  }
}

#[async_trait]
impl RuntimeQuoteStrategy<Clock> for SimulationStrategy {}

#[async_trait]
impl TransactionSyntax for SimulationStrategy {}
