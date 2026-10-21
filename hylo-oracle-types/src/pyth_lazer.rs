/// Pyth Lazer feed id (Lazer's numeric identifier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythLazerFeedId(pub u32);

/// Pyth Lazer channel id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PythLazerChannelId(pub u8);

/// `fixed_rate@1000ms`, equal to
/// `pyth_lazer_protocol::ChannelId::FIXED_RATE_1000`.
pub const LAZER_CHANNEL: PythLazerChannelId = PythLazerChannelId(4);
