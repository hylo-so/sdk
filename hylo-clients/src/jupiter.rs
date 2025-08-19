use anchor_lang::{prelude::Pubkey, AnchorDeserialize};
use anchor_spl::token::Mint;
use anyhow::{anyhow, Result};
use fix::prelude::*;
use hylo_core::exchange_context::ExchangeContext;
use hylo_core::fee_controller::{FeeExtract, LevercoinFees, StablecoinFees};
use hylo_core::pyth::OracleConfig;
use hylo_core::stability_mode::StabilityController;
use hylo_core::total_sol_cache::TotalSolCache;
use jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
  SwapAndAccountMetas, SwapParams,
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::exchange::accounts::{Hylo, LstHeader};
use crate::hylo_exchange;
use crate::pda;
use crate::util::{
  fee_pct_decimal, get_account, JITOSOL_MINT, SOL_USD_PYTH_FEED,
};

#[derive(Clone)]
pub struct HyloExchangeState {
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

impl HyloExchangeState {
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
      self.levercoin_mint()?,
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

  fn quote_from_token_pair(
    &self,
    in_amount: UFix64<N9>,
    input_mint: Pubkey,
    output_mint: Pubkey,
  ) -> Result<Quote> {
    let ctx = self.load_exchange_ctx()?;
    match (input_mint, output_mint) {
      (JITOSOL_MINT, token) if token == *pda::HYUSD => {
        let jitosol_price = self.jitosol_header()?.price_sol.into();
        let FeeExtract {
          fees_extracted,
          amount_remaining,
        } = ctx.stablecoin_mint_fee(&jitosol_price, in_amount)?;
        let stablecoin_nav = ctx.stablecoin_nav()?;
        let hyusd_out = ctx
          .token_conversion(&jitosol_price)?
          .lst_to_token(amount_remaining, stablecoin_nav)?;
        Ok(Quote {
          in_amount: in_amount.bits,
          out_amount: hyusd_out.bits,
          fee_amount: fees_extracted.bits,
          fee_mint: input_mint,
          fee_pct: fee_pct_decimal(fees_extracted, in_amount)?,
        })
      }
      (JITOSOL_MINT, token) if token == *pda::XSOL => {
        let jitosol_price = self.jitosol_header()?.price_sol.into();
        let FeeExtract {
          fees_extracted,
          amount_remaining,
        } = ctx.levercoin_mint_fee(&jitosol_price, in_amount)?;
        let levercoin_mint_nav = ctx.levercoin_mint_nav()?;
        let xsol_out = ctx
          .token_conversion(&jitosol_price)?
          .lst_to_token(amount_remaining, levercoin_mint_nav)?;
        Ok(Quote {
          in_amount: in_amount.bits,
          out_amount: xsol_out.bits,
          fee_amount: fees_extracted.bits,
          fee_mint: input_mint,
          fee_pct: fee_pct_decimal(fees_extracted, in_amount)?,
        })
      }
      _ => Err(anyhow!("Unsupported quote pair")),
    }
  }
}

impl Amm for HyloExchangeState {
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
      Into::<UFixValue64>::into(hylo.oracle_conf_tolerance).try_into()?,
    );
    let stability_controller = StabilityController::new(
      Into::<UFixValue64>::into(hylo.stability_threshold_1).try_into()?,
      Into::<UFixValue64>::into(hylo.stability_threshold_2).try_into()?,
    )?;
    Ok(HyloExchangeState {
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
    hylo_exchange::ID
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
    let stablecoin_mint: Mint = get_account(account_map, &pda::HYUSD)?;
    let levercoin_mint: Mint = get_account(account_map, &pda::XSOL)?;
    let jitosol_header: LstHeader =
      get_account(account_map, &pda::lst_header(JITOSOL_MINT))?;
    let sol_usd: PriceUpdateV2 = get_account(account_map, &SOL_USD_PYTH_FEED)?;
    self.stablecoin_mint = Some(stablecoin_mint);
    self.levercoin_mint = Some(levercoin_mint);
    self.jitosol_header = Some(jitosol_header);
    self.sol_usd = Some(sol_usd);
    Ok(())
  }

  fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
    self.quote_from_token_pair(
      UFix64::new(quote_params.amount),
      quote_params.input_mint,
      quote_params.output_mint,
    )
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
  use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
  use anchor_lang::solana_program::sysvar::clock::{self, Clock};
  use jupiter_amm_interface::SwapMode;

  async fn load_account_map(pubkeys: &[Pubkey]) -> Result<AccountMap> {
    let rpc_url = std::env::var("RPC_URL")?;
    let client = RpcClient::new(rpc_url);
    let accounts = client.get_multiple_accounts(pubkeys).await?;
    pubkeys
      .iter()
      .zip(accounts)
      .map(|(pubkey, account)| {
        account
          .ok_or_else(|| anyhow!("Account not found: {pubkey}"))
          .map(|acc| (*pubkey, acc))
      })
      .collect::<Result<AccountMap>>()
  }

  async fn load_amm_context(client: &RpcClient) -> Result<AmmContext> {
    let clock_account = client.get_account(&clock::ID).await?;
    let clock: Clock = bincode::deserialize(&clock_account.data)?;
    let clock_ref = ClockRef::from(clock);
    Ok(AmmContext { clock_ref })
  }

  #[tokio::test]
  async fn quote() -> Result<()> {
    let rpc_url = std::env::var("RPC_URL")?;
    let client = RpcClient::new(rpc_url);
    let account = client.get_account(&pda::HYLO).await?;

    // Jupiter's KeyedAccount expects data as base64 string
    let jupiter_account = jupiter_amm_interface::KeyedAccount {
      key: *pda::HYLO,
      account,
      params: None,
    };
    let amm_context = load_amm_context(&client).await?;
    let mut exchange =
      HyloExchangeState::from_keyed_account(&jupiter_account, &amm_context)?;
    let accounts_to_update = exchange.get_accounts_to_update();
    let account_map = load_account_map(&accounts_to_update).await?;
    exchange.update(&account_map)?;
    let quote_params = QuoteParams {
      amount: UFix64::<N9>::one().bits,
      input_mint: JITOSOL_MINT,
      output_mint: *pda::HYUSD,
      swap_mode: SwapMode::ExactIn,
    };
    let quote = exchange.quote(&quote_params)?;
    println!("{quote:?}");
    Ok(())
  }
}
