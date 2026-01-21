//! Quote strategy using protocol state.
//!
//! Computes quotes using protocol state and SDK machinery like
//! `ExchangeContext`, without requiring transaction simulation.

mod exchange;
mod stability_pool;

use async_trait::async_trait;
use hylo_core::solana_clock::SolanaClock;

use crate::protocol_state::StateProvider;
use crate::runtime_quote_strategy::RuntimeQuoteStrategy;

pub struct ProtocolStateStrategy<S> {
  pub(crate) state_provider: S,
}

impl<S> ProtocolStateStrategy<S> {
  #[must_use]
  pub fn new(state_provider: S) -> Self {
    Self { state_provider }
  }
}

#[async_trait]
impl<S: StateProvider<C> + Sync, C: SolanaClock> RuntimeQuoteStrategy<C>
  for ProtocolStateStrategy<S>
{
}
