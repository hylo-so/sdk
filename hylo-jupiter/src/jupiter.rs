use std::marker::PhantomData;

use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{anyhow, Context, Result};
use hylo_core::idl::exchange::accounts::{Hylo, LstHeader};
use hylo_core::idl::stability_pool::accounts::PoolConfig;
use hylo_core::idl::tokens::{
  TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL,
};
use hylo_core::idl::{exchange, pda, stability_pool};
use hylo_core::pyth::SOL_USD_PYTH_FEED;
use hylo_jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
  SwapAndAccountMetas, SwapParams,
};
use hylo_quotes::protocol_state::ProtocolState;
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
    *pda::HYLO
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
        Ok(account_metas::mint_stablecoin(user, JITOSOL::MINT))
      }
      (HYUSD::MINT, JITOSOL::MINT) => {
        Ok(account_metas::redeem_stablecoin(user, JITOSOL::MINT))
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
    *pda::HYLO
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
        Ok(account_metas::mint_stablecoin(user, HYLOSOL::MINT))
      }
      (HYUSD::MINT, HYLOSOL::MINT) => {
        Ok(account_metas::redeem_stablecoin(user, HYLOSOL::MINT))
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
    *pda::HYLO
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
        Ok(account_metas::mint_levercoin(user, JITOSOL::MINT))
      }
      (XSOL::MINT, JITOSOL::MINT) => {
        Ok(account_metas::redeem_levercoin(user, JITOSOL::MINT))
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
    *pda::HYLO
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
        Ok(account_metas::mint_levercoin(user, HYLOSOL::MINT))
      }
      (XSOL::MINT, HYLOSOL::MINT) => {
        Ok(account_metas::redeem_levercoin(user, HYLOSOL::MINT))
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
    *pda::HYLO
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
        Ok(account_metas::swap_stable_to_lever(user))
      }
      (XSOL::MINT, HYUSD::MINT) => {
        Ok(account_metas::swap_lever_to_stable(user))
      }
      _ => Err(anyhow!("Invalid mint pair")),
    }
  }
}

impl PairConfig<HYUSD, SHYUSD> for HyloJupiterPair<HYUSD, SHYUSD> {
  fn program_id() -> Pubkey {
    stability_pool::ID
  }
  fn label() -> &'static str {
    "Hylo HYUSD<->SHYUSD"
  }
  fn key() -> Pubkey {
    *pda::POOL_CONFIG
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
      (HYUSD::MINT, SHYUSD::MINT) => {
        Ok(account_metas::stability_pool_deposit(user))
      }
      (SHYUSD::MINT, HYUSD::MINT) => {
        Ok(account_metas::stability_pool_withdraw(user))
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
      *pda::HYLO,
      HYUSD::MINT,
      XSOL::MINT,
      pda::lst_header(JITOSOL::MINT),
      pda::lst_header(HYLOSOL::MINT),
      SOL_USD_PYTH_FEED,
      SHYUSD::MINT,
      *pda::HYUSD_POOL,
      *pda::XSOL_POOL,
      *pda::POOL_CONFIG,
    ]
  }

  fn update(&mut self, account_map: &AccountMap) -> Result<()> {
    let hylo: Hylo = account_map_get(account_map, &pda::HYLO)?;
    let hyusd_mint: Mint = account_map_get(account_map, &HYUSD::MINT)?;
    let xsol_mint: Mint = account_map_get(account_map, &XSOL::MINT)?;
    let jitosol_header: LstHeader =
      account_map_get(account_map, &pda::lst_header(JITOSOL::MINT))?;
    let hylosol_header: LstHeader =
      account_map_get(account_map, &pda::lst_header(HYLOSOL::MINT))?;
    let sol_usd: PriceUpdateV2 =
      account_map_get(account_map, &SOL_USD_PYTH_FEED)?;
    let shyusd_mint: Mint = account_map_get(account_map, &SHYUSD::MINT)?;
    let hyusd_pool: TokenAccount =
      account_map_get(account_map, &pda::HYUSD_POOL)?;
    let xsol_pool: TokenAccount =
      account_map_get(account_map, &pda::XSOL_POOL)?;
    let pool_config: PoolConfig =
      account_map_get(account_map, &pda::POOL_CONFIG)?;

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
    MintArgs, RedeemArgs, StabilityPoolArgs, SwapArgs, TransactionSyntax,
    HYUSD, JITOSOL, SHYUSD, XSOL,
  };
  use hylo_clients::program_client::ProgramClient;
  use hylo_clients::util::{
    build_test_exchange_client, build_test_stability_pool_client,
  };
  use hylo_core::idl::exchange::events::{
    MintLevercoinEventV2, MintStablecoinEventV2, RedeemLevercoinEventV2,
    RedeemStablecoinEventV2, SwapLeverToStableEventV1,
    SwapStableToLeverEventV1,
  };
  use hylo_core::idl::stability_pool::events::{
    UserDepositEvent, UserWithdrawEventV1,
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
    let hylo = build_test_exchange_client()?;
    let args = hylo
      .build_transaction_data::<JITOSOL, HYUSD>(MintArgs {
        amount: amount_lst,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<MintStablecoinEventV2>(&tx)
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
    let hylo = build_test_exchange_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, JITOSOL>(RedeemArgs {
        amount: amount_hyusd,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<RedeemStablecoinEventV2>(&tx)
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
    let hylo = build_test_exchange_client()?;
    let args = hylo
      .build_transaction_data::<JITOSOL, XSOL>(MintArgs {
        amount: amount_lst,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<MintLevercoinEventV2>(&tx)
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
    let hylo = build_test_exchange_client()?;
    let args = hylo
      .build_transaction_data::<XSOL, JITOSOL>(RedeemArgs {
        amount: amount_xsol,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<RedeemLevercoinEventV2>(&tx)
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
    let hylo = build_test_exchange_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, XSOL>(SwapArgs {
        amount: amount_hyusd,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<SwapStableToLeverEventV1>(&tx)
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
    let hylo = build_test_exchange_client()?;
    let args = hylo
      .build_transaction_data::<XSOL, HYUSD>(SwapArgs {
        amount: amount_xsol,
        user: TESTER,
        slippage_config: None,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<SwapLeverToStableEventV1>(&tx)
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
    let hylo = build_test_stability_pool_client()?;
    let args = hylo
      .build_transaction_data::<HYUSD, SHYUSD>(StabilityPoolArgs {
        amount: amount_hyusd,
        user: TESTER,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<UserDepositEvent>(&tx)
      .await?;
    let quote = jup.quote(&quote_params)?;

    // Input
    assert_eq!(sim.stablecoin_deposited.bits, quote.in_amount);

    // Output
    assert_eq!(sim.lp_token_minted.bits, quote.out_amount);

    // Fees extracted
    assert_eq!(0, quote.fee_amount);

    // Fee percentage
    assert_eq!(Decimal::ZERO, quote.fee_pct);
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
    let hylo = build_test_stability_pool_client()?;
    let args = hylo
      .build_transaction_data::<SHYUSD, HYUSD>(StabilityPoolArgs {
        amount: amount_shyusd,
        user: TESTER,
      })
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<UserWithdrawEventV1>(&tx)
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
