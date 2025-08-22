use anchor_lang::prelude::{AnchorDeserialize, Pubkey};
use anchor_spl::token::Mint;
use anyhow::{anyhow, Result};
use fix::prelude::*;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fee_controller::{LevercoinFees, StablecoinFees};
use hylo_core::idl::exchange;
use hylo_core::idl_type_bridge::convert_ufixvalue64;
use hylo_core::pyth::OracleConfig;
use hylo_core::stability_mode::StabilityController;
use hylo_core::total_sol_cache::TotalSolCache;
use jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
  SwapAndAccountMetas, SwapParams,
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::quote;
use crate::util::{account_map_get, JITOSOL_MINT};
use hylo_core::idl::exchange::accounts::{Hylo, LstHeader};
use hylo_core::idl::pda;
use hylo_core::pyth::SOL_USD_PYTH_FEED;

#[derive(Clone)]
pub struct HyloJupiterClient {
  clock: ClockRef,
  total_sol_cache: TotalSolCache,
  stability_controller: StabilityController,
  oracle_config: OracleConfig<N8>,
  stablecoin_fees: StablecoinFees,
  levercoin_fees: LevercoinFees,
  stablecoin_mint: Option<Mint>,
  levercoin_mint: Option<Mint>,
  jitosol_header: Option<LstHeader>,
  sol_usd: Option<PriceUpdateV2>,
}

impl HyloJupiterClient {
  fn load_exchange_ctx(&self) -> Result<ExchangeContext<ClockRef>> {
    let ctx = ExchangeContext::load(
      self.clock.clone(),
      &self.total_sol_cache,
      self.stability_controller,
      self.oracle_config,
      self.stablecoin_fees,
      self.levercoin_fees,
      self.sol_usd()?,
      self.stablecoin_mint()?,
      self.levercoin_mint().ok(),
    )?;
    Ok(ctx)
  }

  fn sol_usd(&self) -> Result<&PriceUpdateV2> {
    self.sol_usd.as_ref().ok_or(anyhow!("`sol_usd` not set"))
  }

  fn stablecoin_mint(&self) -> Result<&Mint> {
    self
      .stablecoin_mint
      .as_ref()
      .ok_or(anyhow!("`stablecoin_mint` not set"))
  }

  fn levercoin_mint(&self) -> Result<&Mint> {
    self
      .levercoin_mint
      .as_ref()
      .ok_or(anyhow!("`levercoin_mint` not set"))
  }

  fn jitosol_header(&self) -> Result<&LstHeader> {
    self
      .jitosol_header
      .as_ref()
      .ok_or(anyhow!("`jitosol_header` not set"))
  }
}

impl Amm for HyloJupiterClient {
  fn from_keyed_account(
    keyed_account: &KeyedAccount,
    amm_context: &AmmContext,
  ) -> Result<Self>
  where
    Self: Sized,
  {
    // Account data is already raw bytes, skip the 8-byte Anchor discriminator
    let hylo = Hylo::try_from_slice(&keyed_account.account.data[8..])?;
    let oracle_config = OracleConfig::new(
      hylo.oracle_interval_secs,
      convert_ufixvalue64(hylo.oracle_conf_tolerance).try_into()?,
    );
    let stability_controller = StabilityController::new(
      convert_ufixvalue64(hylo.stability_threshold_1).try_into()?,
      convert_ufixvalue64(hylo.stability_threshold_2).try_into()?,
    )?;
    Ok(HyloJupiterClient {
      clock: amm_context.clock_ref.clone(),
      total_sol_cache: hylo.total_sol_cache.into(),
      stability_controller,
      oracle_config,
      stablecoin_fees: hylo.stablecoin_fees.into(),
      levercoin_fees: hylo.levercoin_fees.into(),
      stablecoin_mint: None,
      levercoin_mint: None,
      jitosol_header: None,
      sol_usd: None,
    })
  }

  fn label(&self) -> String {
    "Hylo Exchange".to_string()
  }

  fn program_id(&self) -> Pubkey {
    exchange::ID
  }

  fn key(&self) -> Pubkey {
    *pda::HYLO
  }

  fn get_reserve_mints(&self) -> Vec<Pubkey> {
    vec![*pda::HYUSD, *pda::XSOL, JITOSOL_MINT]
  }

  fn get_accounts_to_update(&self) -> Vec<Pubkey> {
    vec![
      *pda::HYUSD,
      *pda::XSOL,
      pda::lst_header(JITOSOL_MINT),
      SOL_USD_PYTH_FEED,
    ]
  }

  fn update(&mut self, account_map: &AccountMap) -> Result<()> {
    let stablecoin_mint: Mint = account_map_get(account_map, &pda::HYUSD)?;
    let levercoin_mint: Mint = account_map_get(account_map, &pda::XSOL)?;
    let jitosol_header: LstHeader =
      account_map_get(account_map, &pda::lst_header(JITOSOL_MINT))?;
    let sol_usd: PriceUpdateV2 =
      account_map_get(account_map, &SOL_USD_PYTH_FEED)?;
    self.stablecoin_mint = Some(stablecoin_mint);
    self.levercoin_mint = Some(levercoin_mint);
    self.jitosol_header = Some(jitosol_header);
    self.sol_usd = Some(sol_usd);
    Ok(())
  }

  fn quote(
    &self,
    QuoteParams {
      amount,
      input_mint,
      output_mint,
      swap_mode: _,
    }: &QuoteParams,
  ) -> Result<Quote> {
    let ctx = self.load_exchange_ctx()?;
    match (*input_mint, *output_mint) {
      (JITOSOL_MINT, token) if token == *pda::HYUSD => {
        quote::hyusd_mint(&ctx, self.jitosol_header()?, UFix64::new(*amount))
      }
      (token, JITOSOL_MINT) if token == *pda::HYUSD => {
        quote::hyusd_redeem(&ctx, self.jitosol_header()?, UFix64::new(*amount))
      }
      (JITOSOL_MINT, token) if token == *pda::XSOL => {
        quote::xsol_mint(&ctx, self.jitosol_header()?, UFix64::new(*amount))
      }
      (token, JITOSOL_MINT) if token == *pda::XSOL => {
        quote::xsol_redeem(&ctx, self.jitosol_header()?, UFix64::new(*amount))
      }
      (token_in, token_out) => {
        if token_in == *pda::HYUSD && token_out == *pda::XSOL {
          quote::hyusd_xsol_swap(&ctx, UFix64::new(*amount))
        } else if token_in == *pda::XSOL && token_out == *pda::HYUSD {
          quote::xsol_hyusd_swap(&ctx, UFix64::new(*amount))
        } else {
          Err(anyhow!("Unsupported quote pair"))
        }
      }
    }
  }

  fn get_swap_and_account_metas(
    &self,
    _swap_params: &SwapParams,
  ) -> Result<SwapAndAccountMetas> {
    todo!()
  }

  fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
    Box::new(self.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use crate::util::{fee_pct_decimal, load_account_map, load_amm_context};

  use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
  use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
  use anchor_client::solana_sdk::signature::Keypair;
  use anchor_client::Cluster;
  use anchor_lang::pubkey;
  use fix::typenum::U9;
  use hylo_clients::exchange_client::ExchangeClient;
  use hylo_clients::program_client::ProgramClient;
  use hylo_core::idl::exchange::events::{
    MintLevercoinEventV2, MintStablecoinEventV2, RedeemLevercoinEventV2,
    RedeemStablecoinEventV2,
  };
  use jupiter_amm_interface::{KeyedAccount, SwapMode};

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
      let fee_pct = fee_pct_decimal::<U9>(
        convert_ufixvalue64($sim.fees_deposited).try_into()?,
        UFix64::new($quote.in_amount),
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
      let fee_pct = fee_pct_decimal::<U9>(
        convert_ufixvalue64($sim.fees_deposited).try_into()?,
        UFix64::new(total_out),
      )?;
      assert_eq!(fee_pct, $quote.fee_pct);
    };
  }

  const TESTER: Pubkey =
    pubkey!("GUX587fnbnZmqmq2hnav8r6siLczKS8wrp9QZRhuWeai");

  async fn build_exchange_client() -> Result<ExchangeClient> {
    let client = ExchangeClient::new_from_keypair(
      Cluster::Mainnet,
      Keypair::new(),
      CommitmentConfig::confirmed(),
    )?;
    Ok(client)
  }

  async fn build_jupiter_client() -> Result<HyloJupiterClient> {
    let url = std::env::var("RPC_URL")?;
    let client = RpcClient::new(url);
    let account = client.get_account(&pda::HYLO).await?;
    let jupiter_account = KeyedAccount {
      key: *pda::HYLO,
      account,
      params: None,
    };
    let amm_context = load_amm_context(&client).await?;
    let mut hylo =
      HyloJupiterClient::from_keyed_account(&jupiter_account, &amm_context)?;
    let accounts_to_update = hylo.get_accounts_to_update();
    let account_map = load_account_map(&client, &accounts_to_update).await?;
    hylo.update(&account_map)?;
    Ok(hylo)
  }

  #[tokio::test]
  async fn mint_hyusd_check() -> Result<()> {
    let amount_lst = UFix64::<N9>::one();
    let quote_params = QuoteParams {
      amount: amount_lst.bits,
      input_mint: JITOSOL_MINT,
      output_mint: *pda::HYUSD,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_client().await?;
    let hylo = build_exchange_client().await?;
    let quote = jup.quote(&quote_params)?;
    let args = hylo
      .mint_stablecoin_args(amount_lst, JITOSOL_MINT, TESTER, None)
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<MintStablecoinEventV2>(tx)
      .await?;
    assert_mint!(sim, quote);
    Ok(())
  }

  #[tokio::test]
  async fn redeem_hyusd_check() -> Result<()> {
    let amount_stablecoin = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_stablecoin.bits,
      input_mint: *pda::HYUSD,
      output_mint: JITOSOL_MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_client().await?;
    let hylo = build_exchange_client().await?;
    let quote = jup.quote(&quote_params)?;
    let args = hylo
      .redeem_stablecoin_args(amount_stablecoin, JITOSOL_MINT, TESTER, None)
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<RedeemStablecoinEventV2>(tx)
      .await?;
    assert_redeem!(sim, quote);
    Ok(())
  }

  #[tokio::test]
  async fn mint_xsol_check() -> Result<()> {
    let amount_lst = UFix64::<N9>::one();
    let quote_params = QuoteParams {
      amount: amount_lst.bits,
      input_mint: JITOSOL_MINT,
      output_mint: *pda::XSOL,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_client().await?;
    let hylo = build_exchange_client().await?;
    let quote = jup.quote(&quote_params)?;
    let args = hylo
      .mint_levercoin_args(amount_lst, JITOSOL_MINT, TESTER, None)
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<MintLevercoinEventV2>(tx)
      .await?;
    assert_mint!(sim, quote);
    Ok(())
  }

  #[tokio::test]
  async fn redeem_xsol_check() -> Result<()> {
    let amount_xsol = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_xsol.bits,
      input_mint: *pda::XSOL,
      output_mint: JITOSOL_MINT,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_client().await?;
    let hylo = build_exchange_client().await?;
    let quote = jup.quote(&quote_params)?;
    let args = hylo
      .redeem_levercoin_args(amount_xsol, JITOSOL_MINT, TESTER, None)
      .await?;
    let tx = hylo.build_simulation_transaction(&TESTER, &args).await?;
    let sim = hylo
      .simulate_transaction_event::<RedeemLevercoinEventV2>(tx)
      .await?;
    assert_redeem!(sim, quote);
    Ok(())
  }

  #[tokio::test]
  async fn hyusd_xsol_swap_check() -> Result<()> {
    let amount_hyusd = UFix64::<N6>::one();
    let quote_params = QuoteParams {
      amount: amount_hyusd.bits,
      input_mint: *pda::XSOL,
      output_mint: *pda::HYUSD,
      swap_mode: SwapMode::ExactIn,
    };
    let jup = build_jupiter_client().await?;
    let hylo = build_exchange_client().await?;
    let quote = jup.quote(&quote_params)?;
    todo!("hyusd_xsol_swap_check");
    Ok(())
  }
}
