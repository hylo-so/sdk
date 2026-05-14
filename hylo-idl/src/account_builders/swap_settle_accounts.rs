//! Composite account types for USDC swap instructions which self-CPI into
//! `settle_rebalance_pnl_{lst,exo}`.

use anchor_lang::prelude::AccountMeta;
use anchor_lang::ToAccountMetas;

use crate::exchange::client::accounts::{
  SettleRebalancePnlExo, SettleRebalancePnlLst, SwapExoToUsdc, SwapLstToUsdc,
  SwapUsdcToExo, SwapUsdcToLst,
};

pub struct SwapLstToUsdcWithSettle {
  pub swap: SwapLstToUsdc,
  pub settle: SettleRebalancePnlLst,
}

impl ToAccountMetas for SwapLstToUsdcWithSettle {
  fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
    [
      self.swap.to_account_metas(is_signer),
      self.settle.to_account_metas(is_signer),
    ]
    .concat()
  }
}

pub struct SwapUsdcToLstWithSettle {
  pub swap: SwapUsdcToLst,
  pub settle: SettleRebalancePnlLst,
}

impl ToAccountMetas for SwapUsdcToLstWithSettle {
  fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
    [
      self.swap.to_account_metas(is_signer),
      self.settle.to_account_metas(is_signer),
    ]
    .concat()
  }
}

pub struct SwapExoToUsdcWithSettle {
  pub swap: SwapExoToUsdc,
  pub settle: SettleRebalancePnlExo,
}

impl ToAccountMetas for SwapExoToUsdcWithSettle {
  fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
    [
      self.swap.to_account_metas(is_signer),
      self.settle.to_account_metas(is_signer),
    ]
    .concat()
  }
}

pub struct SwapUsdcToExoWithSettle {
  pub swap: SwapUsdcToExo,
  pub settle: SettleRebalancePnlExo,
}

impl ToAccountMetas for SwapUsdcToExoWithSettle {
  fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
    [
      self.swap.to_account_metas(is_signer),
      self.settle.to_account_metas(is_signer),
    ]
    .concat()
  }
}
