use anchor_lang::prelude::*;

use crate::{PythLazerChannelId, PythLazerFeedId};

/// Price source backing an observation. New sources append here.
#[derive(
  AnchorSerialize,
  AnchorDeserialize,
  InitSpace,
  Clone,
  Copy,
  Debug,
  PartialEq,
  Eq,
)]
pub enum OracleSource {
  PythLazer,
}

pub enum OracleSourceConfig {
  PythLazer {
    feed_id: PythLazerFeedId,
    channel_id: PythLazerChannelId,
  },
}
