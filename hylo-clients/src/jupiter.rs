use anchor_lang::AccountDeserialize;
use anchor_lang::{prelude::Pubkey, AnchorDeserialize};
use anchor_spl::token::TokenAccount;
use anyhow::anyhow;
use anyhow::Result;
use base64::prelude::{Engine, BASE64_STANDARD};
use fix::prelude::*;
use jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams,
  SwapAndAccountMetas, SwapParams,
};

use crate::exchange::accounts::{Hylo, LstHeader, PriceUpdateV2};
use crate::exchange::types::{LevercoinFees, StablecoinFees};
use crate::hylo_exchange;
use crate::pda;
use crate::util::{JITOSOL_MINT, SOL_USD_PYTH_FEED};

fn get_account<A: AccountDeserialize>(
  account_map: &AccountMap,
  key: &Pubkey,
) -> Result<A> {
  let account = account_map
    .get(key)
    .ok_or(anyhow!("Account not found {}", key))?;
  let decoded = BASE64_STANDARD.decode(&account.data)?;
  let mut bytes = decoded.as_slice();
  let out = A::try_deserialize(&mut bytes)?;
  Ok(out)
}

pub struct HyloExchangeState {
  pub total_sol: UFix64<N9>,
  pub sol_usd_price: Option<UFix64<N8>>,
  pub stablecoin_supply: Option<UFix64<N6>>,
  pub levercoin_supply: Option<UFix64<N6>>,
  pub stablecoin_fees: StablecoinFees,
  pub levercoin_fees: LevercoinFees,
}

impl Amm for HyloExchangeState {
  fn from_keyed_account(
    keyed_account: &KeyedAccount,
    _amm_context: &AmmContext,
  ) -> Result<Self>
  where
    Self: Sized,
  {
    let bytes = BASE64_STANDARD.decode(keyed_account.account.data.clone())?;
    let hylo = Hylo::try_from_slice(&bytes)?;
    let total_sol: UFixValue64 = hylo.total_sol_cache.total_sol.into();
    Ok(HyloExchangeState {
      total_sol: total_sol.try_into()?,
      sol_usd_price: None,
      stablecoin_supply: None,
      levercoin_supply: None,
      stablecoin_fees: hylo.stablecoin_fees,
      levercoin_fees: hylo.levercoin_fees,
    })
  }

  fn label(&self) -> String {
    "Hylo Exchange".to_string()
  }

  fn program_id(&self) -> Pubkey {
    hylo_exchange::ID
  }

  fn key(&self) -> Pubkey {
    pda::hylo()
  }

  fn get_reserve_mints(&self) -> Vec<Pubkey> {
    vec![pda::hyusd(), pda::xsol(), JITOSOL_MINT]
  }

  fn get_accounts_to_update(&self) -> Vec<Pubkey> {
    vec![
      pda::hyusd(),
      pda::xsol(),
      pda::lst_header(JITOSOL_MINT),
      SOL_USD_PYTH_FEED,
    ]
  }

  fn update(&mut self, account_map: &AccountMap) -> Result<()> {
    let hyusd: TokenAccount = get_account(account_map, &pda::hyusd())?;
    let xsol: TokenAccount = get_account(account_map, &pda::xsol())?;
    let jitosol_header: LstHeader =
      get_account(account_map, &pda::lst_header(JITOSOL_MINT))?;
    let sol_usd: PriceUpdateV2 = get_account(account_map, &SOL_USD_PYTH_FEED)?;
    todo!()
  }

  fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
    todo!()
  }

  fn get_swap_and_account_metas(
    &self,
    swap_params: &SwapParams,
  ) -> Result<SwapAndAccountMetas> {
    todo!()
  }

  fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
    todo!()
  }
}
