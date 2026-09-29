pub const MICROS_PER_SECOND: u64 = 1_000_000;

/// Supported price-exponent range (inclusive). MUST stay in sync with the
/// consumer contract `hylo_core::oracle::normalize_price`, which maps Pyth
/// exponents `-9..=-2` to canonical fixed-point precision and rejects anything
/// else — an observation outside this range would be unusable downstream, so we
/// reject it at write time rather than admit a dead price.
pub const MIN_EXPONENT: i16 = -9;
pub const MAX_EXPONENT: i16 = -2;
