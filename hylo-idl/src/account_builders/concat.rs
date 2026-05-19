use anchor_lang::prelude::AccountMeta;
use anchor_lang::ToAccountMetas;

/// Concatenates two [`ToAccountMetas`] sets.
pub struct Concat<A, B> {
  pub a: A,
  pub b: B,
}

impl<A, B> Concat<A, B> {
  pub fn new(a: A, b: B) -> Concat<A, B> {
    Concat { a, b }
  }
}

impl<A: ToAccountMetas, B: ToAccountMetas> ToAccountMetas for Concat<A, B> {
  fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
    [
      self.a.to_account_metas(is_signer),
      self.b.to_account_metas(is_signer),
    ]
    .concat()
  }
}
