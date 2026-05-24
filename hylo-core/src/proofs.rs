use fix::prelude::*;
use fix::typenum::Integer;

#[must_use]
pub fn any_ufix64<Exp: Integer>() -> UFix64<Exp> {
  UFix64::new(kani::any())
}

#[must_use]
pub fn token_amount<Exp: Integer>() -> UFix64<Exp> {
  let v: UFix64<Exp> = any_ufix64();
  kani::assume(v.bits < (1u64 << 40));
  v
}

#[must_use]
pub fn tolerance() -> UFix64<N4> {
  let v: UFix64<N4> = any_ufix64();
  kani::assume(v <= UFix64::one());
  v
}

#[must_use]
pub fn any_ifix64<Exp: Integer>() -> IFix64<Exp> {
  IFix64::new(kani::any())
}

#[must_use]
pub fn bounded_ifix64<Exp: Integer>() -> IFix64<Exp> {
  let v: IFix64<Exp> = any_ifix64();
  kani::assume(v.bits >= -(1i64 << 24));
  kani::assume(v.bits < (1i64 << 24));
  v
}
