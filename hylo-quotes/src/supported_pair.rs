//! Supported token pair marker trait.

use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

/// Marker trait indicating a token pair is supported.
///
/// This is a sealed trait - only this crate can implement it. Used as a
/// type-level constraint to ensure only valid pairs can be used.
pub trait SupportedPair<IN: TokenMint, OUT: TokenMint>:
  private::Sealed
{
}

mod private {
  pub trait Sealed {}
  impl<IN: super::TokenMint, OUT: super::TokenMint> Sealed for (IN, OUT) {}
}

impl SupportedPair<JITOSOL, HYUSD> for (JITOSOL, HYUSD) {}
impl SupportedPair<HYUSD, JITOSOL> for (HYUSD, JITOSOL) {}
impl SupportedPair<HYLOSOL, HYUSD> for (HYLOSOL, HYUSD) {}
impl SupportedPair<HYUSD, HYLOSOL> for (HYUSD, HYLOSOL) {}
impl SupportedPair<JITOSOL, XSOL> for (JITOSOL, XSOL) {}
impl SupportedPair<XSOL, JITOSOL> for (XSOL, JITOSOL) {}
impl SupportedPair<HYLOSOL, XSOL> for (HYLOSOL, XSOL) {}
impl SupportedPair<XSOL, HYLOSOL> for (XSOL, HYLOSOL) {}
impl SupportedPair<HYUSD, XSOL> for (HYUSD, XSOL) {}
impl SupportedPair<XSOL, HYUSD> for (XSOL, HYUSD) {}
impl SupportedPair<HYUSD, SHYUSD> for (HYUSD, SHYUSD) {}
