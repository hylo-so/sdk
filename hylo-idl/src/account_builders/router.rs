use crate::router::client::accounts::Route;
use crate::{exchange, stability_pool};

#[must_use]
pub fn route() -> Route {
  Route {
    hylo_exchange: exchange::ID,
    hylo_stability_pool: stability_pool::ID,
  }
}
