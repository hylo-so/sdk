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

impl From<crate::stability_pool::types::UFixValue64> for UFixValue64 {
  fn from(idl: crate::stability_pool::types::UFixValue64) -> Self {
    UFixValue64 {
      bits: idl.bits,
      exp: idl.exp,
    }
  }
}

impl<Exp: Integer> TryFrom<crate::stability_pool::types::UFixValue64>
  for UFix64<Exp>
{
  type Error = anchor_lang::error::Error;

  fn try_from(
    idl: crate::stability_pool::types::UFixValue64,
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
