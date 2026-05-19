use std::marker::PhantomData;
use std::sync::Arc;

use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Context, Result};
use fix::prelude::{UFix64, N8};
use hylo_core::exchange_context::ExoExchangeContext;
use hylo_core::idl::earn_pool::accounts::PoolConfig;
use hylo_core::idl::exchange::accounts::{ExoPair, Hylo, LstHeader, UsdcPair};
use hylo_core::idl::tokens::{
  StakePool, TokenMint, CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC,
  XSOL,
};
use hylo_core::idl::{earn_pool, exchange, pda};
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::pyth::{query_pyth_oracle, OracleConfig, SOL_USD};
use hylo_jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
  SwapAndAccountMetas, SwapParams,
};
use hylo_quotes::protocol_state::{ProtocolState, UsdcExchangeState};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::account_metas;
use crate::util::{account_map_get, quote, validate_swap_params};

/// Bidirectional single-pair Jupiter AMM client.
pub struct HyloJupiterPair<IN, OUT>
where
  IN: TokenMint,
  OUT: TokenMint,
{
  clock: ClockRef,
  state: Option<ProtocolState<ClockRef>>,
  _phantom: PhantomData<(IN, OUT)>,
}

impl<IN: TokenMint, OUT: TokenMint> Clone for HyloJupiterPair<IN, OUT> {
  fn clone(&self) -> Self {
    Self {
      clock: self.clock.clone(),
      state: self.state.clone(),
      _phantom: PhantomData,
    }
  }
}

/// Pair-specific configuration and dispatch.
pub trait PairConfig<IN: TokenMint, OUT: TokenMint> {
  fn program_id() -> Pubkey;
  fn label() -> &'static str;
  fn key() -> Pubkey;

  /// Generate a quote for the given pair.
  ///
  /// # Errors
  /// * Unsupported pair
  /// * Arithmetic error
  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote>;

  /// Return related accounts for one direction of the pair.
  ///
  /// # Errors
  /// * Unsupported pair
  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas>;
}

impl PairConfig<JITOSOL, HYUSD> for HyloJupiterPair<JITOSOL, HYUSD> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo JITOSOL<->HYUSD"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (JITOSOL::MINT, HYUSD::MINT) => quote::<JITOSOL, HYUSD>(state, amount),
      (HYUSD::MINT, JITOSOL::MINT) => quote::<HYUSD, JITOSOL>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (JITOSOL::MINT, HYUSD::MINT) => {
        Ok(account_metas::mint_stablecoin_lst(user, JITOSOL::MINT))
      }
      (HYUSD::MINT, JITOSOL::MINT) => {
        Ok(account_metas::redeem_stablecoin_lst(user, JITOSOL::MINT))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<HYLOSOL, HYUSD> for HyloJupiterPair<HYLOSOL, HYUSD> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo HYLOSOL<->HYUSD"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (HYLOSOL::MINT, HYUSD::MINT) => quote::<HYLOSOL, HYUSD>(state, amount),
      (HYUSD::MINT, HYLOSOL::MINT) => quote::<HYUSD, HYLOSOL>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (HYLOSOL::MINT, HYUSD::MINT) => {
        Ok(account_metas::mint_stablecoin_lst(user, HYLOSOL::MINT))
      }
      (HYUSD::MINT, HYLOSOL::MINT) => {
        Ok(account_metas::redeem_stablecoin_lst(user, HYLOSOL::MINT))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<JITOSOL, XSOL> for HyloJupiterPair<JITOSOL, XSOL> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo JITOSOL<->XSOL"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (JITOSOL::MINT, XSOL::MINT) => quote::<JITOSOL, XSOL>(state, amount),
      (XSOL::MINT, JITOSOL::MINT) => quote::<XSOL, JITOSOL>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (JITOSOL::MINT, XSOL::MINT) => {
        Ok(account_metas::mint_levercoin_lst(user, JITOSOL::MINT))
      }
      (XSOL::MINT, JITOSOL::MINT) => {
        Ok(account_metas::redeem_levercoin_lst(user, JITOSOL::MINT))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<HYLOSOL, XSOL> for HyloJupiterPair<HYLOSOL, XSOL> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo HYLOSOL<->XSOL"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (HYLOSOL::MINT, XSOL::MINT) => quote::<HYLOSOL, XSOL>(state, amount),
      (XSOL::MINT, HYLOSOL::MINT) => quote::<XSOL, HYLOSOL>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (HYLOSOL::MINT, XSOL::MINT) => {
        Ok(account_metas::mint_levercoin_lst(user, HYLOSOL::MINT))
      }
      (XSOL::MINT, HYLOSOL::MINT) => {
        Ok(account_metas::redeem_levercoin_lst(user, HYLOSOL::MINT))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<HYUSD, XSOL> for HyloJupiterPair<HYUSD, XSOL> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo HYUSD<->XSOL"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (HYUSD::MINT, XSOL::MINT) => quote::<HYUSD, XSOL>(state, amount),
      (XSOL::MINT, HYUSD::MINT) => quote::<XSOL, HYUSD>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (HYUSD::MINT, XSOL::MINT) => {
        Ok(account_metas::convert_stable_to_lever_lst(user))
      }
      (XSOL::MINT, HYUSD::MINT) => {
        Ok(account_metas::convert_lever_to_stable_lst(user))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<HYUSD, SHYUSD> for HyloJupiterPair<HYUSD, SHYUSD> {
  fn program_id() -> Pubkey {
    earn_pool::ID
  }
  fn label() -> &'static str {
    "Hylo HYUSD<->SHYUSD"
  }
  fn key() -> Pubkey {
    pda::POOL_CONFIG
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (HYUSD::MINT, SHYUSD::MINT) => quote::<HYUSD, SHYUSD>(state, amount),
      (SHYUSD::MINT, HYUSD::MINT) => quote::<SHYUSD, HYUSD>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (HYUSD::MINT, SHYUSD::MINT) => Ok(account_metas::earn_pool_deposit(user)),
      (SHYUSD::MINT, HYUSD::MINT) => {
        Ok(account_metas::earn_pool_withdraw(user))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<JITOSOL, HYLOSOL> for HyloJupiterPair<JITOSOL, HYLOSOL> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo JITOSOL<->HYLOSOL"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (JITOSOL::MINT, HYLOSOL::MINT) => {
        quote::<JITOSOL, HYLOSOL>(state, amount)
      }
      (HYLOSOL::MINT, JITOSOL::MINT) => {
        quote::<HYLOSOL, JITOSOL>(state, amount)
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (JITOSOL::MINT, HYLOSOL::MINT) => Ok(account_metas::swap_lst_to_lst(
        user,
        JITOSOL::MINT,
        HYLOSOL::MINT,
      )),
      (HYLOSOL::MINT, JITOSOL::MINT) => Ok(account_metas::swap_lst_to_lst(
        user,
        HYLOSOL::MINT,
        JITOSOL::MINT,
      )),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<JITOSOL, USDC> for HyloJupiterPair<JITOSOL, USDC> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo JITOSOL<->USDC"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (JITOSOL::MINT, USDC::MINT) => quote::<JITOSOL, USDC>(state, amount),
      (USDC::MINT, JITOSOL::MINT) => quote::<USDC, JITOSOL>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (JITOSOL::MINT, USDC::MINT) => Ok(account_metas::swap_lst_to_usdc(
        user,
        JITOSOL::MINT,
        JITOSOL::POOL_STATE,
      )),
      (USDC::MINT, JITOSOL::MINT) => Ok(account_metas::swap_usdc_to_lst(
        user,
        JITOSOL::MINT,
        JITOSOL::POOL_STATE,
      )),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<HYLOSOL, USDC> for HyloJupiterPair<HYLOSOL, USDC> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo HYLOSOL<->USDC"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (HYLOSOL::MINT, USDC::MINT) => quote::<HYLOSOL, USDC>(state, amount),
      (USDC::MINT, HYLOSOL::MINT) => quote::<USDC, HYLOSOL>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (HYLOSOL::MINT, USDC::MINT) => Ok(account_metas::swap_lst_to_usdc(
        user,
        HYLOSOL::MINT,
        HYLOSOL::POOL_STATE,
      )),
      (USDC::MINT, HYLOSOL::MINT) => Ok(account_metas::swap_usdc_to_lst(
        user,
        HYLOSOL::MINT,
        HYLOSOL::POOL_STATE,
      )),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<USDC, HYUSD> for HyloJupiterPair<USDC, HYUSD> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo USDC<->HYUSD"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (USDC::MINT, HYUSD::MINT) => quote::<USDC, HYUSD>(state, amount),
      (HYUSD::MINT, USDC::MINT) => quote::<HYUSD, USDC>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (USDC::MINT, HYUSD::MINT) => {
        Ok(account_metas::mint_stablecoin_usdc(user))
      }
      (HYUSD::MINT, USDC::MINT) => {
        Ok(account_metas::redeem_stablecoin_usdc(user))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<CBBTC, USDC> for HyloJupiterPair<CBBTC, USDC> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo CBBTC<->USDC"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (CBBTC::MINT, USDC::MINT) => quote::<CBBTC, USDC>(state, amount),
      (USDC::MINT, CBBTC::MINT) => quote::<USDC, CBBTC>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (CBBTC::MINT, USDC::MINT) => Ok(account_metas::swap_exo_to_usdc(
        user,
        CBBTC::MINT,
        pda::BTC_USD_PYTH_FEED,
      )),
      (USDC::MINT, CBBTC::MINT) => Ok(account_metas::swap_usdc_to_exo(
        user,
        CBBTC::MINT,
        pda::BTC_USD_PYTH_FEED,
      )),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<CBBTC, HYUSD> for HyloJupiterPair<CBBTC, HYUSD> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo CBBTC<->HYUSD"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (CBBTC::MINT, HYUSD::MINT) => quote::<CBBTC, HYUSD>(state, amount),
      (HYUSD::MINT, CBBTC::MINT) => quote::<HYUSD, CBBTC>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (CBBTC::MINT, HYUSD::MINT) => Ok(account_metas::mint_stablecoin_exo(
        user,
        CBBTC::MINT,
        pda::BTC_USD_PYTH_FEED,
      )),
      (HYUSD::MINT, CBBTC::MINT) => Ok(account_metas::redeem_stablecoin_exo(
        user,
        CBBTC::MINT,
        pda::BTC_USD_PYTH_FEED,
      )),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<CBBTC, XBTC> for HyloJupiterPair<CBBTC, XBTC> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo CBBTC<->XBTC"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (CBBTC::MINT, XBTC::MINT) => quote::<CBBTC, XBTC>(state, amount),
      (XBTC::MINT, CBBTC::MINT) => quote::<XBTC, CBBTC>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (CBBTC::MINT, XBTC::MINT) => Ok(account_metas::mint_levercoin_exo(
        user,
        CBBTC::MINT,
        pda::BTC_USD_PYTH_FEED,
      )),
      (XBTC::MINT, CBBTC::MINT) => Ok(account_metas::redeem_levercoin_exo(
        user,
        CBBTC::MINT,
        pda::BTC_USD_PYTH_FEED,
      )),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<HYUSD, XBTC> for HyloJupiterPair<HYUSD, XBTC> {
  fn program_id() -> Pubkey {
    exchange::ID
  }
  fn label() -> &'static str {
    "Hylo HYUSD<->XBTC"
  }
  fn key() -> Pubkey {
    pda::HYLO
  }

  fn quote(
    state: &ProtocolState<ClockRef>,
    amount: u64,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    match (input_mint, output_mint) {
      (HYUSD::MINT, XBTC::MINT) => quote::<HYUSD, XBTC>(state, amount),
      (XBTC::MINT, HYUSD::MINT) => quote::<XBTC, HYUSD>(state, amount),
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }

  fn build_account_metas(
    user: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    match (input_mint, output_mint) {
      (HYUSD::MINT, XBTC::MINT) => {
        Ok(account_metas::convert_stable_to_lever_exo(
          user,
          CBBTC::MINT,
          pda::BTC_USD_PYTH_FEED,
        ))
      }
      (XBTC::MINT, HYUSD::MINT) => {
        Ok(account_metas::convert_lever_to_stable_exo(
          user,
          CBBTC::MINT,
          pda::BTC_USD_PYTH_FEED,
        ))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl<IN, OUT> Amm for HyloJupiterPair<IN, OUT>
where
  IN: TokenMint + 'static,
  OUT: TokenMint + 'static,
  Self: PairConfig<IN, OUT> + Clone + Send + Sync,
{
  fn from_keyed_account(
    _keyed_account: &KeyedAccount,
    amm_context: &AmmContext,
  ) -> Result<Self>
  where
    Self: Sized,
  {
    Ok(HyloJupiterPair {
      clock: amm_context.clock_ref.clone(),
      state: None,
      _phantom: PhantomData,
    })
  }

  fn label(&self) -> String {
    <Self as PairConfig<IN, OUT>>::label().to_string()
  }

  fn program_id(&self) -> Pubkey {
    <Self as PairConfig<IN, OUT>>::program_id()
  }

  fn key(&self) -> Pubkey {
    <Self as PairConfig<IN, OUT>>::key()
  }

  fn get_reserve_mints(&self) -> Vec<Pubkey> {
    vec![IN::MINT, OUT::MINT]
  }

  fn get_accounts_to_update(&self) -> Vec<Pubkey> {
    vec![
      pda::HYLO,
      HYUSD::MINT,
      XSOL::MINT,
      pda::lst_header(JITOSOL::MINT),
      pda::lst_header(HYLOSOL::MINT),
      JITOSOL::POOL_STATE,
      HYLOSOL::POOL_STATE,
      SOL_USD.address,
      SHYUSD::MINT,
      pda::HYUSD_POOL,
      pda::XSOL_POOL,
      pda::POOL_CONFIG,
      pda::exo_pair(CBBTC::MINT),
      pda::exo_vault(CBBTC::MINT),
      pda::exo_levercoin_mint(CBBTC::MINT),
      pda::BTC_USD_PYTH_FEED,
      pda::USDC_PAIR,
      pda::USDC_USD_PYTH_FEED,
    ]
  }

  fn update(&mut self, account_map: &AccountMap) -> Result<()> {
    // Core protocol state
    let hylo: Hylo = account_map_get(account_map, &pda::HYLO)?;
    let hyusd_mint: Mint = account_map_get(account_map, &HYUSD::MINT)?;
    let xsol_mint: Mint = account_map_get(account_map, &XSOL::MINT)?;
    let jitosol_header: LstHeader =
      account_map_get(account_map, &pda::lst_header(JITOSOL::MINT))?;
    let hylosol_header: LstHeader =
      account_map_get(account_map, &pda::lst_header(HYLOSOL::MINT))?;
    let sol_usd: PriceUpdateV2 =
      account_map_get(account_map, &SOL_USD.address)?;

    // Earn pool
    let shyusd_mint: Mint = account_map_get(account_map, &SHYUSD::MINT)?;
    let hyusd_pool: TokenAccount =
      account_map_get(account_map, &pda::HYUSD_POOL)?;
    let xsol_pool: TokenAccount =
      account_map_get(account_map, &pda::XSOL_POOL)?;
    let pool_config: PoolConfig =
      account_map_get(account_map, &pda::POOL_CONFIG)?;

    // cbBTC exo context
    let exo_pair: ExoPair =
      account_map_get(account_map, &pda::exo_pair(CBBTC::MINT))?;
    let cbbtc_vault: TokenAccount =
      account_map_get(account_map, &pda::exo_vault(CBBTC::MINT))?;
    let xbtc_mint: Mint =
      account_map_get(account_map, &pda::exo_levercoin_mint(CBBTC::MINT))?;
    let btc_usd: PriceUpdateV2 =
      account_map_get(account_map, &pda::BTC_USD_PYTH_FEED)?;
    let usdc_pair: UsdcPair = account_map_get(account_map, &pda::USDC_PAIR)?;
    let usdc_usd: PriceUpdateV2 =
      account_map_get(account_map, &pda::USDC_USD_PYTH_FEED)?;
    let exo_oracle_config = OracleConfig::new(
      exo_pair.oracle_interval_secs,
      exo_pair.oracle_conf_tolerance.try_into()?,
    );
    let total_collateral = UFix64::<N8>::new(cbbtc_vault.amount)
      .checked_convert()
      .context("cbBTC vault N8->N9 overflow")?;
    let cbbtc_exchange_context = Arc::new(
      ExoExchangeContext::load(
        self.clock.clone(),
        total_collateral,
        exo_pair.stablecoin_mint_threshold.try_into()?,
        exo_oracle_config,
        exo_pair.levercoin_fees.into(),
        &btc_usd,
        exo_pair.virtual_stablecoin.into(),
        Some(&xbtc_mint),
        exo_pair.rebalance_deviation_tolerance.try_into()?,
        exo_pair.sell_curve_config.into(),
        exo_pair.buy_curve_config.into(),
        exo_pair.levercoin_market_cap_limit.try_into()?,
      )
      .context("ExoExchangeContext::load")?,
    );

    // USDC exchange state
    let usdc_oracle_config = OracleConfig::new(
      usdc_pair.oracle_interval_secs,
      usdc_pair.oracle_conf_tolerance.try_into()?,
    );
    let usdc_oracle =
      query_pyth_oracle(&self.clock, &usdc_usd, usdc_oracle_config)?;
    let usdc_exchange_state = UsdcExchangeState {
      usdc_usd_price: usdc_oracle.price_range()?,
      swap_fee: usdc_pair.swap_fee.try_into()?,
    };

    // Stake pools
    let jitosol_pool_state = account_map
      .get(&JITOSOL::POOL_STATE)
      .context("JitoSOL pool state not found")?;
    let jitosol_stake_pool =
      SplStakePool::from_bytes(&jitosol_pool_state.data)?;
    let hylosol_pool_state = account_map
      .get(&HYLOSOL::POOL_STATE)
      .context("hyloSOL pool state not found")?;
    let hylosol_stake_pool =
      SplStakePool::from_bytes(&hylosol_pool_state.data)?;

    self.state = Some(ProtocolState::build(
      self.clock.clone(),
      &hylo,
      jitosol_header,
      hylosol_header,
      hyusd_mint,
      xsol_mint,
      shyusd_mint,
      pool_config,
      hyusd_pool,
      xsol_pool,
      &sol_usd,
      cbbtc_exchange_context,
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
    )?);

    Ok(())
  }

  fn quote(&self, params: &QuoteParams) -> Result<Quote> {
    let state = self.state.as_ref().context("`state` not set")?;
    <Self as PairConfig<IN, OUT>>::quote(
      state,
      params.amount,
      params.input_mint,
      params.output_mint,
    )
  }

  fn get_swap_and_account_metas(
    &self,
    p: &SwapParams,
  ) -> Result<SwapAndAccountMetas> {
    let SwapParams {
      source_mint,
      destination_mint,
      token_transfer_authority: user,
      ..
    } = validate_swap_params(p)?;
    <Self as PairConfig<IN, OUT>>::build_account_metas(
      *user,
      *source_mint,
      *destination_mint,
    )
  }

  fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
    Box::new(self.clone())
  }
}

#[cfg(test)]
mod tests {
  use anchor_lang::pubkey;
  use fix::prelude::*;
  use hylo_clients::prelude::{
    RouterArgs, TransactionSyntax, CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD,
    USDC, XBTC, XSOL,
  };
  use hylo_clients::program_client::ProgramClient;
  use hylo_clients::util::build_test_router_client;
  use hylo_core::idl::earn_pool::events::{
    UserDepositEvent, UserWithdrawEvent,
  };
  use hylo_core::idl::exchange::events::{
    ConvertLeverToStableExoEvent, ConvertLeverToStableLstEvent,
    ConvertStableToLeverExoEvent, ConvertStableToLeverLstEvent,
    MintLevercoinExoEvent, MintLevercoinLstEvent, MintStablecoinExoEvent,
    MintStablecoinLstEvent, MintStablecoinUsdcEvent, RedeemLevercoinExoEvent,
    RedeemLevercoinLstEvent, RedeemStablecoinExoEvent,
    RedeemStablecoinLstEvent, RedeemStablecoinUsdcEvent, SwapExoToUsdcEvent,
    SwapLstToLstEvent, SwapLstToUsdcEvent, SwapUsdcToExoEvent,
    SwapUsdcToLstEvent,
  };
  use hylo_jupiter_amm_interface::{KeyedAccount, SwapMode};
  use rust_decimal::Decimal;
  use solana_rpc_client::nonblocking::rpc_client::RpcClient;

  use super::*;
  use crate::util::{fee_pct_decimal, load_account_map, load_amm_context};

  macro_rules! assert_mint {
    ($sim:expr, $quote:expr) => {
      // Input amount
      assert_eq!(
        $sim
          .collateral_deposited
          .bits
          .checked_add($sim.fees_deposited.bits),
        Some($quote.in_amount)
      );

      // Output amount
      assert_eq!($sim.minted.bits, $quote.out_amount);

      // Fees extracted
      assert_eq!($sim.fees_deposited.bits, $quote.fee_amount);

      // Fee percentage
      let fee_pct = fee_pct_decimal(
        $sim.fees_deposited.try_into()?,
        UFix64::<N9>::new($quote.in_amount),
      )?;
      assert_eq!(fee_pct, $quote.fee_pct);
    };
  }

  macro_rules! assert_redeem {
    ($sim:expr, $quote:expr) => {
      // Input amount
      assert_eq!($sim.redeemed.bits, $quote.in_amount);

      // Output amount
      assert_eq!($sim.collateral_withdrawn.bits, $quote.out_amount);

      // Fees extracted
      assert_eq!($sim.fees_deposited.bits, $quote.fee_amount);

      // Fee percentage
      let total_out = $sim
        .collateral_withdrawn
        .bits
        .checked_add($sim.fees_deposited.bits)
        .ok_or(anyhow!("assert_redeem fee percentage"))?;
      let fee_pct = fee_pct_decimal(
        $sim.fees_deposited.try_into()?,
        UFix64::<N9>::new(total_out),
      )?;
      assert_eq!(fee_pct, $quote.fee_pct);
    };
  }

  const TESTER: Pubkey =
    pubkey!("GUX587fnbnZmqmq2hnav8r6siLczKS8wrp9QZRhuWeai");

  async fn build_jupiter_pair<IN, OUT>() -> Result<HyloJupiterPair<IN, OUT>>
  where
    IN: TokenMint + 'static,
    OUT: TokenMint + 'static,
    HyloJupiterPair<IN, OUT>: PairConfig<IN, OUT> + Clone + Send + Sync,
  {
    let url = std::env::var("RPC_URL")?;
    let client = RpcClient::new(url);
    let key = <HyloJupiterPair<IN, OUT> as PairConfig<IN, OUT>>::key();
    let account = client.get_account(&key).await?;
    let jupiter_account = KeyedAccount {
      key,
      account,
      params: None,
    };
    let amm_context = load_amm_context(&client).await?;
    let mut pair = HyloJupiterPair::<IN, OUT>::from_keyed_account(
      &jupiter_account,
      &amm_context,
    )?;
    let accounts_to_update = pair.get_accounts_to_update();
    let account_map = load_account_map(&client, &accounts_to_update).await?;
    pair.update(&account_map)?;
    Ok(pair)
  }

  #[tokio::test]
  async fn mint_hyusd_check() -> Result<()> {
    let amount_lst = UFix64::<N9>::one();
    let quote_params = QuoteParams {
      amount: amount_lst.bits,
      input_mint: JITOSOL::MINT,
      output_mint: HYUSD::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<JITOSOL, HYUSD>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<JITOSOL, HYUSD>(RouterArgs {
        amount: amount_lst.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<MintStablecoinLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_mint!(sim, quote);
    Ok(())
  }

  #[tokio::test]
  async fn redeem_hyusd_check() -> Result<()> {
    let amount_hyusd = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_hyusd.bits,
      input_mint: HYUSD::MINT,
      output_mint: JITOSOL::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<JITOSOL, HYUSD>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, JITOSOL>(RouterArgs {
        amount: amount_hyusd.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<RedeemStablecoinLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_redeem!(sim, quote);
    Ok(())
  }

  #[tokio::test]
  async fn mint_xsol_check() -> Result<()> {
    let amount_lst = UFix64::<N9>::one();
    let quote_params = QuoteParams {
      amount: amount_lst.bits,
      input_mint: JITOSOL::MINT,
      output_mint: XSOL::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<JITOSOL, XSOL>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<JITOSOL, XSOL>(RouterArgs {
        amount: amount_lst.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<MintLevercoinLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_mint!(sim, quote);
    Ok(())
  }

  #[tokio::test]
  async fn redeem_xsol_check() -> Result<()> {
    let amount_xsol = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_xsol.bits,
      input_mint: XSOL::MINT,
      output_mint: JITOSOL::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<JITOSOL, XSOL>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<XSOL, JITOSOL>(RouterArgs {
        amount: amount_xsol.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<RedeemLevercoinLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_redeem!(sim, quote);
    Ok(())
  }

  #[tokio::test]
  async fn hyusd_xsol_swap_check() -> Result<()> {
    let amount_hyusd = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_hyusd.bits,
      input_mint: HYUSD::MINT,
      output_mint: XSOL::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<HYUSD, XSOL>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, XSOL>(RouterArgs {
        amount: amount_hyusd.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<ConvertStableToLeverLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;

    // Input
    let fees: UFix64<N6> = sim.stablecoin_fees.try_into()?;
    let burned = sim.stablecoin_burned.try_into()?;
    let total_in = fees.checked_add(&burned).ok_or(anyhow!("total_in"))?;
    assert_eq!(total_in.bits, quote.in_amount);

    // Output
    assert_eq!(sim.levercoin_minted.bits, quote.out_amount);

    // Fees extracted
    assert_eq!(sim.stablecoin_fees.bits, quote.fee_amount);

    // Fee percentage
    let fee_pct = fee_pct_decimal(fees, total_in)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn xsol_hyusd_swap_check() -> Result<()> {
    let amount_xsol = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_xsol.bits,
      input_mint: XSOL::MINT,
      output_mint: HYUSD::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<HYUSD, XSOL>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<XSOL, HYUSD>(RouterArgs {
        amount: amount_xsol.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<ConvertLeverToStableLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;

    // Input
    assert_eq!(sim.levercoin_burned.bits, quote.in_amount);

    // Output
    assert_eq!(sim.stablecoin_minted_user.bits, quote.out_amount);

    // Fees extracted
    assert_eq!(sim.stablecoin_minted_fees.bits, quote.fee_amount);

    // Fee percentage
    let fees: UFix64<N6> = sim.stablecoin_minted_fees.try_into()?;
    let out = sim.stablecoin_minted_user.try_into()?;
    let total_in = fees.checked_add(&out).ok_or(anyhow!("total_in"))?;
    let fee_pct = fee_pct_decimal(fees, total_in)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn shyusd_mint_check() -> Result<()> {
    let amount_hyusd = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_hyusd.bits,
      input_mint: HYUSD::MINT,
      output_mint: SHYUSD::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<HYUSD, SHYUSD>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, SHYUSD>(RouterArgs {
        amount: amount_hyusd.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<UserDepositEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;

    // Input
    assert_eq!(sim.stablecoin_deposited.bits, quote.in_amount);

    // Output
    assert_eq!(sim.lp_token_minted.bits, quote.out_amount);

    // Fees extracted
    assert_eq!(u64::MIN, quote.fee_amount);

    // Fee percentage
    assert_eq!(Decimal::ZERO, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn jitosol_to_hylosol_swap_check() -> Result<()> {
    let amount_in = UFix64::<N9>::one();
    let quote_params = QuoteParams {
      amount: amount_in.bits,
      input_mint: JITOSOL::MINT,
      output_mint: HYLOSOL::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<JITOSOL, HYLOSOL>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<JITOSOL, HYLOSOL>(RouterArgs {
        amount: amount_in.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<SwapLstToLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_eq!(sim.lst_a_in.bits, quote.in_amount);
    assert_eq!(sim.lst_b_out.bits, quote.out_amount);
    assert_eq!(sim.lst_a_fees_extracted.bits, quote.fee_amount);
    let fees: UFix64<N9> = sim.lst_a_fees_extracted.try_into()?;
    let total_in: UFix64<N9> = sim.lst_a_in.try_into()?;
    let fee_pct = fee_pct_decimal(fees, total_in)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn hylosol_to_jitosol_swap_check() -> Result<()> {
    let amount_in = UFix64::<N9>::one();
    let quote_params = QuoteParams {
      amount: amount_in.bits,
      input_mint: HYLOSOL::MINT,
      output_mint: JITOSOL::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<JITOSOL, HYLOSOL>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<HYLOSOL, JITOSOL>(RouterArgs {
        amount: amount_in.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<SwapLstToLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_eq!(sim.lst_a_in.bits, quote.in_amount);
    assert_eq!(sim.lst_b_out.bits, quote.out_amount);
    assert_eq!(sim.lst_a_fees_extracted.bits, quote.fee_amount);
    let fees: UFix64<N9> = sim.lst_a_fees_extracted.try_into()?;
    let total_in: UFix64<N9> = sim.lst_a_in.try_into()?;
    let fee_pct = fee_pct_decimal(fees, total_in)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn jitosol_to_usdc_swap_check() -> Result<()> {
    let amount_in = UFix64::<N9>::one();
    let quote_params = QuoteParams {
      amount: amount_in.bits,
      input_mint: JITOSOL::MINT,
      output_mint: USDC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<JITOSOL, USDC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<JITOSOL, USDC>(RouterArgs {
        amount: amount_in.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<SwapLstToUsdcEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_eq!(sim.lst_deposited.bits, quote.in_amount);
    let usdc_withdrawn: UFix64<N9> = sim.usdc_withdrawn.try_into()?;
    let out: UFix64<N6> = usdc_withdrawn.checked_convert().context("N9->N6")?;
    assert_eq!(out.bits, quote.out_amount);
    assert_eq!(u64::MIN, quote.fee_amount);
    assert_eq!(Decimal::ZERO, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn usdc_to_jitosol_swap_check() -> Result<()> {
    let amount_in = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_in.bits,
      input_mint: USDC::MINT,
      output_mint: JITOSOL::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<JITOSOL, USDC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<USDC, JITOSOL>(RouterArgs {
        amount: amount_in.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<SwapUsdcToLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let usdc_deposited: UFix64<N9> = sim.usdc_deposited.try_into()?;
    let in_amt: UFix64<N6> =
      usdc_deposited.checked_convert().context("N9->N6")?;
    assert_eq!(in_amt.bits, quote.in_amount);
    assert_eq!(sim.lst_withdrawn.bits, quote.out_amount);
    assert_eq!(u64::MIN, quote.fee_amount);
    assert_eq!(Decimal::ZERO, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn hylosol_to_usdc_swap_check() -> Result<()> {
    let amount_in = UFix64::<N9>::one();
    let quote_params = QuoteParams {
      amount: amount_in.bits,
      input_mint: HYLOSOL::MINT,
      output_mint: USDC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<HYLOSOL, USDC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<HYLOSOL, USDC>(RouterArgs {
        amount: amount_in.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<SwapLstToUsdcEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_eq!(sim.lst_deposited.bits, quote.in_amount);
    let usdc_withdrawn: UFix64<N9> = sim.usdc_withdrawn.try_into()?;
    let out: UFix64<N6> = usdc_withdrawn.checked_convert().context("N9->N6")?;
    assert_eq!(out.bits, quote.out_amount);
    assert_eq!(u64::MIN, quote.fee_amount);
    assert_eq!(Decimal::ZERO, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn usdc_to_hylosol_swap_check() -> Result<()> {
    let amount_in = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_in.bits,
      input_mint: USDC::MINT,
      output_mint: HYLOSOL::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<HYLOSOL, USDC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<USDC, HYLOSOL>(RouterArgs {
        amount: amount_in.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<SwapUsdcToLstEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let usdc_deposited: UFix64<N9> = sim.usdc_deposited.try_into()?;
    let in_amt: UFix64<N6> =
      usdc_deposited.checked_convert().context("N9->N6")?;
    assert_eq!(in_amt.bits, quote.in_amount);
    assert_eq!(sim.lst_withdrawn.bits, quote.out_amount);
    assert_eq!(u64::MIN, quote.fee_amount);
    assert_eq!(Decimal::ZERO, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn usdc_to_hyusd_mint_check() -> Result<()> {
    let amount_usdc = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_usdc.bits,
      input_mint: USDC::MINT,
      output_mint: HYUSD::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<USDC, HYUSD>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<USDC, HYUSD>(RouterArgs {
        amount: amount_usdc.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<MintStablecoinUsdcEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let usdc_deposited: UFix64<N9> = sim.usdc_deposited.try_into()?;
    let usdc_fees: UFix64<N9> = sim.usdc_fees.try_into()?;
    let fee_base = usdc_deposited
      .checked_add(&usdc_fees)
      .ok_or(anyhow!("fee_base"))?;
    let in_amt: UFix64<N6> = fee_base.checked_convert().context("N9->N6")?;
    assert_eq!(in_amt.bits, quote.in_amount);
    assert_eq!(sim.stablecoin_minted.bits, quote.out_amount);
    assert_eq!(sim.usdc_fees.bits, quote.fee_amount);
    let fee_pct = fee_pct_decimal(usdc_fees, fee_base)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn hyusd_to_usdc_redeem_check() -> Result<()> {
    let amount_hyusd = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_hyusd.bits,
      input_mint: HYUSD::MINT,
      output_mint: USDC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<USDC, HYUSD>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, USDC>(RouterArgs {
        amount: amount_hyusd.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<RedeemStablecoinUsdcEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let burned: UFix64<N6> = sim.stablecoin_burned.try_into()?;
    let fees: UFix64<N6> = sim.stablecoin_fees.try_into()?;
    let fee_base = burned.checked_add(&fees).ok_or(anyhow!("fee_base"))?;
    assert_eq!(fee_base.bits, quote.in_amount);
    assert_eq!(sim.usdc_withdrawn.bits, quote.out_amount);
    assert_eq!(sim.stablecoin_fees.bits, quote.fee_amount);
    let fee_pct = fee_pct_decimal(fees, fee_base)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn cbbtc_to_usdc_swap_check() -> Result<()> {
    let amount_cbbtc = UFix64::<N8>::one();
    let quote_params = QuoteParams {
      amount: amount_cbbtc.bits,
      input_mint: CBBTC::MINT,
      output_mint: USDC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<CBBTC, USDC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<CBBTC, USDC>(RouterArgs {
        amount: amount_cbbtc.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<SwapExoToUsdcEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let collateral_deposited: UFix64<N9> =
      sim.collateral_deposited.try_into()?;
    let in_amt: UFix64<N8> =
      collateral_deposited.checked_convert().context("N9->N8")?;
    assert_eq!(in_amt.bits, quote.in_amount);
    let usdc_withdrawn: UFix64<N9> = sim.usdc_withdrawn.try_into()?;
    let out: UFix64<N6> = usdc_withdrawn.checked_convert().context("N9->N6")?;
    assert_eq!(out.bits, quote.out_amount);
    assert_eq!(u64::MIN, quote.fee_amount);
    assert_eq!(Decimal::ZERO, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn usdc_to_cbbtc_swap_check() -> Result<()> {
    let amount_usdc = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_usdc.bits,
      input_mint: USDC::MINT,
      output_mint: CBBTC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<CBBTC, USDC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<USDC, CBBTC>(RouterArgs {
        amount: amount_usdc.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<SwapUsdcToExoEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let usdc_deposited: UFix64<N9> = sim.usdc_deposited.try_into()?;
    let in_amt: UFix64<N6> =
      usdc_deposited.checked_convert().context("N9->N6")?;
    assert_eq!(in_amt.bits, quote.in_amount);
    let collateral_withdrawn: UFix64<N9> =
      sim.collateral_withdrawn.try_into()?;
    let out: UFix64<N8> =
      collateral_withdrawn.checked_convert().context("N9->N8")?;
    assert_eq!(out.bits, quote.out_amount);
    assert_eq!(u64::MIN, quote.fee_amount);
    assert_eq!(Decimal::ZERO, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn cbbtc_to_hyusd_mint_check() -> Result<()> {
    let amount_cbbtc = UFix64::<N8>::one();
    let quote_params = QuoteParams {
      amount: amount_cbbtc.bits,
      input_mint: CBBTC::MINT,
      output_mint: HYUSD::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<CBBTC, HYUSD>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<CBBTC, HYUSD>(RouterArgs {
        amount: amount_cbbtc.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<MintStablecoinExoEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let collateral_deposited: UFix64<N9> =
      sim.collateral_deposited.try_into()?;
    let fees: UFix64<N9> = sim.fees_deposited.try_into()?;
    let fee_base = collateral_deposited
      .checked_add(&fees)
      .ok_or(anyhow!("fee_base"))?;
    let in_amt: UFix64<N8> = fee_base.checked_convert().context("N9->N8")?;
    assert_eq!(in_amt.bits, quote.in_amount);
    assert_eq!(sim.minted.bits, quote.out_amount);
    assert_eq!(sim.fees_deposited.bits, quote.fee_amount);
    let fee_pct = fee_pct_decimal(fees, fee_base)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn hyusd_to_cbbtc_redeem_check() -> Result<()> {
    let amount_hyusd = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_hyusd.bits,
      input_mint: HYUSD::MINT,
      output_mint: CBBTC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<CBBTC, HYUSD>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, CBBTC>(RouterArgs {
        amount: amount_hyusd.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<RedeemStablecoinExoEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_eq!(sim.redeemed.bits, quote.in_amount);
    let collateral_withdrawn: UFix64<N9> =
      sim.collateral_withdrawn.try_into()?;
    let out: UFix64<N8> =
      collateral_withdrawn.checked_convert().context("N9->N8")?;
    assert_eq!(out.bits, quote.out_amount);
    assert_eq!(sim.fees_deposited.bits, quote.fee_amount);
    let fees: UFix64<N9> = sim.fees_deposited.try_into()?;
    let fee_base = collateral_withdrawn
      .checked_add(&fees)
      .ok_or(anyhow!("fee_base"))?;
    let fee_pct = fee_pct_decimal(fees, fee_base)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn cbbtc_to_xbtc_mint_check() -> Result<()> {
    let amount_cbbtc = UFix64::<N8>::one();
    let quote_params = QuoteParams {
      amount: amount_cbbtc.bits,
      input_mint: CBBTC::MINT,
      output_mint: XBTC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<CBBTC, XBTC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<CBBTC, XBTC>(RouterArgs {
        amount: amount_cbbtc.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<MintLevercoinExoEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let collateral_deposited: UFix64<N9> =
      sim.collateral_deposited.try_into()?;
    let fees: UFix64<N9> = sim.fees_deposited.try_into()?;
    let fee_base = collateral_deposited
      .checked_add(&fees)
      .ok_or(anyhow!("fee_base"))?;
    let in_amt: UFix64<N8> = fee_base.checked_convert().context("N9->N8")?;
    assert_eq!(in_amt.bits, quote.in_amount);
    assert_eq!(sim.minted.bits, quote.out_amount);
    assert_eq!(sim.fees_deposited.bits, quote.fee_amount);
    let fee_pct = fee_pct_decimal(fees, fee_base)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn xbtc_to_cbbtc_redeem_check() -> Result<()> {
    let amount_xbtc = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_xbtc.bits,
      input_mint: XBTC::MINT,
      output_mint: CBBTC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<CBBTC, XBTC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<XBTC, CBBTC>(RouterArgs {
        amount: amount_xbtc.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<RedeemLevercoinExoEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_eq!(sim.redeemed.bits, quote.in_amount);
    let collateral_withdrawn: UFix64<N9> =
      sim.collateral_withdrawn.try_into()?;
    let out: UFix64<N8> =
      collateral_withdrawn.checked_convert().context("N9->N8")?;
    assert_eq!(out.bits, quote.out_amount);
    assert_eq!(sim.fees_deposited.bits, quote.fee_amount);
    let fees: UFix64<N9> = sim.fees_deposited.try_into()?;
    let fee_base = collateral_withdrawn
      .checked_add(&fees)
      .ok_or(anyhow!("fee_base"))?;
    let fee_pct = fee_pct_decimal(fees, fee_base)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn hyusd_to_xbtc_convert_check() -> Result<()> {
    let amount_hyusd = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_hyusd.bits,
      input_mint: HYUSD::MINT,
      output_mint: XBTC::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<HYUSD, XBTC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, XBTC>(RouterArgs {
        amount: amount_hyusd.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<ConvertStableToLeverExoEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    let fees: UFix64<N6> = sim.stablecoin_fees.try_into()?;
    let burned: UFix64<N6> = sim.stablecoin_burned.try_into()?;
    let total_in = fees.checked_add(&burned).ok_or(anyhow!("total_in"))?;
    assert_eq!(total_in.bits, quote.in_amount);
    assert_eq!(sim.levercoin_minted.bits, quote.out_amount);
    assert_eq!(sim.stablecoin_fees.bits, quote.fee_amount);
    let fee_pct = fee_pct_decimal(fees, total_in)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn xbtc_to_hyusd_convert_check() -> Result<()> {
    let amount_xbtc = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_xbtc.bits,
      input_mint: XBTC::MINT,
      output_mint: HYUSD::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<HYUSD, XBTC>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<XBTC, HYUSD>(RouterArgs {
        amount: amount_xbtc.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<ConvertLeverToStableExoEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;
    assert_eq!(sim.levercoin_burned.bits, quote.in_amount);
    assert_eq!(sim.stablecoin_minted_user.bits, quote.out_amount);
    assert_eq!(sim.stablecoin_minted_fees.bits, quote.fee_amount);
    let fees: UFix64<N6> = sim.stablecoin_minted_fees.try_into()?;
    let out: UFix64<N6> = sim.stablecoin_minted_user.try_into()?;
    let total_in = fees.checked_add(&out).ok_or(anyhow!("total_in"))?;
    let fee_pct = fee_pct_decimal(fees, total_in)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }

  #[tokio::test]
  async fn shyusd_redeem_check() -> Result<()> {
    let amount_shyusd = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_shyusd.bits,
      input_mint: SHYUSD::MINT,
      output_mint: HYUSD::MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_pair::<HYUSD, SHYUSD>().await?;
    let hylo = build_test_router_client()?;
    let args = hylo
      .build_transaction_data::<SHYUSD, HYUSD>(RouterArgs {
        amount: amount_shyusd.bits,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_return::<UserWithdrawEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;

    // Input
    assert_eq!(sim.lp_token_burned.bits, quote.in_amount);

    // Output
    assert_eq!(sim.stablecoin_withdrawn.bits, quote.out_amount);

    // Fees extracted
    assert_eq!(sim.stablecoin_fees.bits, quote.fee_amount);

    // Fee percentage
    let out = UFix64::<N6>::new(sim.stablecoin_withdrawn.bits);
    let fees = UFix64::<N6>::new(sim.stablecoin_fees.bits);
    let total_hyusd = out.checked_add(&fees).ok_or(anyhow!("total_hyusd"))?;
    let fee_pct = fee_pct_decimal(fees, total_hyusd)?;
    assert_eq!(fee_pct, quote.fee_pct);
    Ok(())
  }
}
