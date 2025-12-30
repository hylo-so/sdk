use hylo_clients::protocol_state::ProtocolState;
use hylo_clients::util::LST;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::exchange::accounts::LstHeader;
use hylo_idl::tokens::{HYLOSOL, JITOSOL};

/// Trait for getting LST header from protocol state.
pub(crate) trait LstProvider<L: LST> {
  fn lst_header(&self) -> &LstHeader;
}

impl<C: SolanaClock> LstProvider<JITOSOL> for ProtocolState<C> {
  fn lst_header(&self) -> &LstHeader {
    &self.jitosol_header
  }
}

impl<C: SolanaClock> LstProvider<HYLOSOL> for ProtocolState<C> {
  fn lst_header(&self) -> &LstHeader {
    &self.hylosol_header
  }
}
