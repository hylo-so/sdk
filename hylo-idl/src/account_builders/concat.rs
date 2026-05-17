//! Concatenates two [`ToAccountMetas`] sets.

use anchor_lang::prelude::AccountMeta;
use anchor_lang::ToAccountMetas;

pub struct Concat<A, B> {
  pub a: A,
  pub b: B,
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
