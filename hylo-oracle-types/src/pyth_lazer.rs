/// Pyth Lazer feed id (Lazer's numeric identifier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythLazerFeedId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythLazerChannelId(pub u8);

/// On-chain Pyth Lazer channel id the mio publisher signs on
/// (`fixed_rate@1000ms`). Equals
/// `pyth_lazer_protocol::ChannelId::FIXED_RATE_1000`; kept a bare `u8` so
/// consumers don't pull the Lazer protocol crate.
pub const LAZER_CHANNEL: PythLazerChannelId = PythLazerChannelId(4);
