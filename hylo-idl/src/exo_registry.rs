use anyhow::{Context, Result};

use crate::router::accounts::ExoRegistry;
use crate::router::types::ExoEntry;

impl ExoRegistry {
  /// Registered entries.
  ///
  /// # Errors
  /// * `current_size` exceeds capacity
  pub fn registered_entries(&self) -> Result<&[ExoEntry]> {
    self
      .entries
      .get(..usize::from(self.current_size))
      .context("Exo registry length exceeds capacity")
  }
}
