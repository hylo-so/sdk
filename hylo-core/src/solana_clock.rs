use anchor_lang::prelude::Clock;
use jupiter_amm_interface::ClockRef;
use std::sync::atomic::Ordering;

/// Abstracts the concept of Solana's onchain clock.
pub trait SolanaClock {
  fn slot(&self) -> u64;
  fn epoch_start_timestamp(&self) -> i64;
  fn epoch(&self) -> u64;
  fn leader_schedule_epoch(&self) -> u64;
  fn unix_timestamp(&self) -> i64;
}

impl SolanaClock for Clock {
  fn slot(&self) -> u64 {
    self.slot
  }

  fn epoch_start_timestamp(&self) -> i64 {
    self.epoch_start_timestamp
  }

  fn epoch(&self) -> u64 {
    self.epoch
  }

  fn leader_schedule_epoch(&self) -> u64 {
    self.leader_schedule_epoch
  }

  fn unix_timestamp(&self) -> i64 {
    self.unix_timestamp
  }
}

impl SolanaClock for ClockRef {
  fn slot(&self) -> u64 {
    self.slot.load(Ordering::Relaxed)
  }

  fn epoch_start_timestamp(&self) -> i64 {
    self.epoch_start_timestamp.load(Ordering::Relaxed)
  }

  fn epoch(&self) -> u64 {
    self.epoch.load(Ordering::Relaxed)
  }

  fn leader_schedule_epoch(&self) -> u64 {
    self.leader_schedule_epoch.load(Ordering::Relaxed)
  }

  fn unix_timestamp(&self) -> i64 {
    self.unix_timestamp.load(Ordering::Relaxed)
  }
}
