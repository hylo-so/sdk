use fix::prelude::{UFix64, UFixValue64};
use fix::typenum::Integer;

impl From<crate::exchange::types::UFixValue64> for UFixValue64 {
  fn from(idl: crate::exchange::types::UFixValue64) -> Self {
    UFixValue64 {
      bits: idl.bits,
      exp: idl.exp,
    }
  }
}

impl<Exp: Integer> TryFrom<crate::exchange::types::UFixValue64>
  for UFix64<Exp>
{
  type Error = anchor_lang::error::Error;

  fn try_from(
    idl: crate::exchange::types::UFixValue64,
  ) -> Result<Self, Self::Error> {
    let value: UFixValue64 = idl.into();
    value.try_into()
  }
}

impl From<crate::earn_pool::types::UFixValue64> for UFixValue64 {
  fn from(idl: crate::earn_pool::types::UFixValue64) -> Self {
    UFixValue64 {
      bits: idl.bits,
      exp: idl.exp,
    }
  }
}

impl<Exp: Integer> TryFrom<crate::earn_pool::types::UFixValue64>
  for UFix64<Exp>
{
  type Error = anchor_lang::error::Error;

  fn try_from(
    idl: crate::earn_pool::types::UFixValue64,
  ) -> Result<Self, Self::Error> {
    let value: UFixValue64 = idl.into();
    value.try_into()
  }
}

impl From<UFixValue64> for crate::exchange::types::UFixValue64 {
  fn from(idl: UFixValue64) -> Self {
    crate::exchange::types::UFixValue64 {
      bits: idl.bits,
      exp: idl.exp,
    }
  }
}

impl From<UFixValue64> for crate::router::types::UFixValue64 {
  fn from(val: UFixValue64) -> Self {
    crate::router::types::UFixValue64 {
      bits: val.bits,
      exp: val.exp,
    }
  }
}

impl From<UFixValue64> for crate::earn_pool::types::UFixValue64 {
  fn from(val: UFixValue64) -> Self {
    crate::earn_pool::types::UFixValue64 {
      bits: val.bits,
      exp: val.exp,
    }
  }
}

impl From<crate::exchange::types::SlippageConfig>
  for crate::router::types::SlippageConfig
{
  fn from(val: crate::exchange::types::SlippageConfig) -> Self {
    let expected: UFixValue64 = val.expected_token_out.into();
    let tolerance: UFixValue64 = val.slippage_tolerance.into();
    crate::router::types::SlippageConfig {
      expected_token_out: expected.into(),
      slippage_tolerance: tolerance.into(),
    }
  }
}

impl From<crate::exchange::types::SlippageConfig>
  for crate::earn_pool::types::SlippageConfig
{
  fn from(val: crate::exchange::types::SlippageConfig) -> Self {
    let expected: UFixValue64 = val.expected_token_out.into();
    let tolerance: UFixValue64 = val.slippage_tolerance.into();
    crate::earn_pool::types::SlippageConfig {
      expected_token_out: expected.into(),
      slippage_tolerance: tolerance.into(),
    }
  }
}
