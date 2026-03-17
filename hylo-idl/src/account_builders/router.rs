use crate::exchange;
use crate::router::client::accounts::Route;

#[must_use]
pub fn route() -> Route {
  Route {
    hylo_exchange: exchange::ID,
  }
}
