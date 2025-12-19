//! Compute unit providers for token pair operations.

use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::SupportedPair;

/// ATA creation compute units (measured via `calibrate_compute_units`)
const ATA_CREATION_CU: u64 = 7_338;

/// Trait for providing compute unit values for token pairs.
pub trait ComputeUnitProvider<IN: TokenMint, OUT: TokenMint>
where
  (IN, OUT): SupportedPair<IN, OUT>,
{
  /// Get default compute units (base, safe) for a token pair.
  fn default_compute_units() -> (u64, u64);
}

/// Compute unit provider for Hylo protocol token pairs.
pub struct HyloComputeUnitProvider;

impl HyloComputeUnitProvider {
  #[must_use]
  pub fn new() -> Self {
    Self
  }
}

impl Default for HyloComputeUnitProvider {
  fn default() -> Self {
    Self::new()
  }
}

// ============================================================================
// Implementations for JITOSOL → HYUSD (mint stablecoin)
// ============================================================================

impl ComputeUnitProvider<JITOSOL, HYUSD> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 92_931; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for HYUSD → JITOSOL (redeem stablecoin)
// ============================================================================

impl ComputeUnitProvider<HYUSD, JITOSOL> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 92_695; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for HYLOSOL → HYUSD (mint stablecoin)
// ============================================================================

impl ComputeUnitProvider<HYLOSOL, HYUSD> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 92_931; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for HYUSD → HYLOSOL (redeem stablecoin)
// ============================================================================

impl ComputeUnitProvider<HYUSD, HYLOSOL> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 94_195; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for JITOSOL → XSOL (mint levercoin)
// ============================================================================

impl ComputeUnitProvider<JITOSOL, XSOL> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 94_617; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for XSOL → JITOSOL (redeem levercoin)
// ============================================================================

impl ComputeUnitProvider<XSOL, JITOSOL> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 95_448; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for HYLOSOL → XSOL (mint levercoin)
// ============================================================================

impl ComputeUnitProvider<HYLOSOL, XSOL> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 95_448; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for XSOL → HYLOSOL (redeem levercoin)
// ============================================================================

impl ComputeUnitProvider<XSOL, HYLOSOL> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 96_948; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for HYUSD → XSOL (swap)
// ============================================================================

impl ComputeUnitProvider<HYUSD, XSOL> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 83_411; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for XSOL → HYUSD (swap)
// ============================================================================

impl ComputeUnitProvider<XSOL, HYUSD> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 82_600; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}

// ============================================================================
// Implementations for HYUSD → SHYUSD (stability pool deposit)
// ============================================================================

impl ComputeUnitProvider<HYUSD, SHYUSD> for HyloComputeUnitProvider {
  fn default_compute_units() -> (u64, u64) {
    const OPERATION_BASE_CU: u64 = 74_011; // measured via `calibrate_compute_units`

    const BASE_TOTAL: u64 = ATA_CREATION_CU + OPERATION_BASE_CU;
    const BASE_WITH_MARGIN: u64 = (BASE_TOTAL * 110) / 100;
    const SAFE_WITH_MARGIN: u64 = (BASE_WITH_MARGIN * 150) / 100;

    (BASE_WITH_MARGIN, SAFE_WITH_MARGIN)
  }
}
