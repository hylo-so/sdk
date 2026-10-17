use std::collections::BTreeSet;
use std::iter::once;

use anchor_lang::prelude::Pubkey;
use anchor_spl::token::{Mint, TokenAccount};
use anyhow::{Context, Result};
use fix::prelude::UFix64;
use hylo_core::error::CoreError;
use hylo_core::idl::earn_pool::accounts::PoolConfig;
use hylo_core::idl::exchange::accounts::{Hylo, LstHeader, UsdcPair};
use hylo_core::idl::router::types::ExoEntry;
use hylo_core::idl::tokens::{
  StakePool, TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XSOL,
};
use hylo_core::idl::{pda, router};
use hylo_core::lst::stake_pool::SplStakePool;
use hylo_core::pyth::{query_pyth_oracle, OracleConfig, SOL_USD};
use hylo_core::reserve_gate::ReserveGate;
use hylo_core::virtual_stablecoin::VirtualStablecoin;
use hylo_jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
  SwapAndAccountMetas, SwapParams,
};
use hylo_quotes::protocol_state::ExoRole::{
  Collateral, Levercoin, Stablecoin, Usdc,
};
use hylo_quotes::protocol_state::{
  exo_entry_role, exo_pubkeys_from_entries, exo_pyth_feed_by_mint,
  find_exo_entry, read_exo_entries, ExoAccounts, ExoPairAccounts,
  ProtocolState, UsdcExchangeState, EXO_PAIR_ACCOUNTS,
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::account_metas;
use crate::util::{
  account_map_get, fee_pct, keyed_account, validate_swap_params,
};

/// Mints of the pairs compiled into the exchange and earn pool programs.
const ENSHRINED_MINTS: [Pubkey; 6] = [
  HYUSD::MINT,
  SHYUSD::MINT,
  XSOL::MINT,
  USDC::MINT,
  JITOSOL::MINT,
  HYLOSOL::MINT,
];

/// Accounts behind the enshrined pairs, fetched on every update.
const ENSHRINED_ACCOUNTS: [Pubkey; 16] = [
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
  pda::POOL_CONFIG,
  pda::USDC_PAIR,
  pda::USDC_USD_PYTH_FEED,
  pda::lst_vault(JITOSOL::MINT),
  pda::lst_vault(HYLOSOL::MINT),
  pda::usdc_vault(USDC::MINT),
];

/// Reads the active exo entries from the router-owned registry account.
///
/// # Errors
/// * Registry account is missing, malformed, or carries an invalid length.
fn exo_entries(account_map: &AccountMap) -> Result<Vec<ExoEntry>> {
  let account = keyed_account(account_map, &pda::EXO_REGISTRY)?;
  read_exo_entries(&account.data)
}

/// Builds the USDC exchange state from a Jupiter account snapshot.
fn usdc_state(
  clock: &ClockRef,
  usdc_pair: &UsdcPair,
  usdc_usd: &PriceUpdateV2,
  usdc_vault: &TokenAccount,
) -> Result<UsdcExchangeState> {
  let usdc_oracle_config = OracleConfig::new(
    usdc_pair.oracle_interval_secs,
    usdc_pair.oracle_conf_tolerance.try_into()?,
  );
  let usdc_usd_spot = query_pyth_oracle(clock, usdc_usd, usdc_oracle_config)
    .map(|oracle| oracle.spot);
  let virtual_stablecoin: VirtualStablecoin =
    usdc_pair.virtual_stablecoin.into();
  let reserve_gate = ReserveGate::validated(
    usdc_pair.reserve_gate.reserve_per_tvl.into(),
    usdc_pair.reserve_gate.tvl_cap.into(),
  )?;

  Ok(UsdcExchangeState {
    mint_fee: usdc_pair.mint_fee.try_into()?,
    redeem_fee: usdc_pair.redeem_fee.try_into()?,
    paused: usdc_pair.paused,
    vault_balance: UFix64::new(usdc_vault.amount),
    virtual_stablecoin,
    usdc_usd_spot,
    par_tolerance: usdc_pair.par_tolerance.into(),
    reserve_gate,
  })
}

/// Jupiter AMM over every Hylo route: the enshrined LST, USDC, and earn
/// pool pairs plus the exo pairs listed in the registry.
#[derive(Clone)]
pub struct HyloJupiter {
  clock: ClockRef,
  state: Option<ProtocolState<ClockRef>>,
  exo_entries: Vec<ExoEntry>,
  exo_account_keys: Vec<Pubkey>,
}

impl HyloJupiter {
  /// Decodes the enshrined accounts and builds the protocol state with
  /// the parsed exo pairs.
  fn build_state(
    &self,
    account_map: &AccountMap,
    exo_accounts: &[ExoAccounts],
  ) -> Result<ProtocolState<ClockRef>> {
    let hylo: Hylo = account_map_get(account_map, &pda::HYLO)?;
    let hyusd_mint: Mint = account_map_get(account_map, &HYUSD::MINT)?;
    let xsol_mint: Mint = account_map_get(account_map, &XSOL::MINT)?;
    let jitosol_header: LstHeader =
      account_map_get(account_map, &pda::lst_header(JITOSOL::MINT))?;
    let hylosol_header: LstHeader =
      account_map_get(account_map, &pda::lst_header(HYLOSOL::MINT))?;
    let sol_usd: PriceUpdateV2 =
      account_map_get(account_map, &SOL_USD.address)?;
    let shyusd_mint: Mint = account_map_get(account_map, &SHYUSD::MINT)?;
    let hyusd_pool: TokenAccount =
      account_map_get(account_map, &pda::HYUSD_POOL)?;
    let pool_config: PoolConfig =
      account_map_get(account_map, &pda::POOL_CONFIG)?;
    let usdc_pair: UsdcPair = account_map_get(account_map, &pda::USDC_PAIR)?;
    let jitosol_vault: TokenAccount =
      account_map_get(account_map, &pda::lst_vault(JITOSOL::MINT))?;
    let hylosol_vault: TokenAccount =
      account_map_get(account_map, &pda::lst_vault(HYLOSOL::MINT))?;
    let usdc_vault: TokenAccount =
      account_map_get(account_map, &pda::usdc_vault(USDC::MINT))?;
    let usdc_usd: PriceUpdateV2 =
      account_map_get(account_map, &pda::USDC_USD_PYTH_FEED)?;
    let usdc_exchange_state =
      usdc_state(&self.clock, &usdc_pair, &usdc_usd, &usdc_vault)?;
    let jitosol_stake_pool = SplStakePool::from_bytes(
      &keyed_account(account_map, &JITOSOL::POOL_STATE)?.data,
    )?;
    let hylosol_stake_pool = SplStakePool::from_bytes(
      &keyed_account(account_map, &HYLOSOL::POOL_STATE)?.data,
    )?;
    ProtocolState::build(
      self.clock.clone(),
      &hylo,
      jitosol_header,
      hylosol_header,
      hyusd_mint,
      xsol_mint,
      shyusd_mint,
      pool_config,
      hyusd_pool,
      &sol_usd,
      exo_accounts,
      usdc_exchange_state,
      jitosol_stake_pool,
      hylosol_stake_pool,
      UFix64::new(jitosol_vault.amount),
      UFix64::new(hylosol_vault.amount),
    )
  }

  /// Parses the pair accounts of `entries` from the map, in the order of
  /// `exo_account_keys`.
  fn exo_accounts(
    &self,
    account_map: &AccountMap,
    entries: &[ExoEntry],
  ) -> Result<Vec<ExoAccounts>> {
    self
      .exo_account_keys
      .chunks(EXO_PAIR_ACCOUNTS)
      .zip(entries)
      .map(|(keys, entry)| {
        let fetched = keys
          .iter()
          .map(|key| account_map.get(key).cloned())
          .collect::<Vec<_>>();
        let raw =
          ExoPairAccounts::from_fetched(&fetched).with_context(|| {
            format!("exo pair accounts for {}", entry.collateral_mint)
          })?;
        ExoAccounts::parse(entry, &raw)
      })
      .collect()
  }

  /// Swap accounts for the exo route between `source` and `destination`.
  fn exo_account_metas(
    &self,
    user: Pubkey,
    source: Pubkey,
    destination: Pubkey,
  ) -> Result<SwapAndAccountMetas> {
    let entry = find_exo_entry(&self.exo_entries, source, destination)
      .ok_or(CoreError::UnroutablePair)?;
    let collateral = entry.collateral_mint;
    let oracle = exo_pyth_feed_by_mint(collateral)?.address;
    let roles = (
      exo_entry_role(entry, source)?,
      exo_entry_role(entry, destination)?,
    );
    match roles {
      (Collateral, Stablecoin) => {
        Ok(account_metas::mint_stablecoin_exo(user, collateral, oracle))
      }
      (Stablecoin, Collateral) => Ok(account_metas::redeem_stablecoin_exo(
        user, collateral, oracle,
      )),
      (Collateral, Levercoin) => {
        Ok(account_metas::mint_levercoin_exo(user, collateral, oracle))
      }
      (Levercoin, Collateral) => Ok(account_metas::redeem_levercoin_exo(
        user, collateral, oracle,
      )),
      (Stablecoin, Levercoin) => Ok(
        account_metas::convert_stable_to_lever_exo(user, collateral, oracle),
      ),
      (Levercoin, Stablecoin) => Ok(
        account_metas::convert_lever_to_stable_exo(user, collateral, oracle),
      ),
      (Collateral, Usdc) => {
        Ok(account_metas::swap_exo_to_usdc(user, collateral, oracle))
      }
      (Usdc, Collateral) => {
        Ok(account_metas::swap_usdc_to_exo(user, collateral, oracle))
      }
      _ => Err(CoreError::UnroutablePair.into()),
    }
  }
}

impl Amm for HyloJupiter {
  fn from_keyed_account(
    _keyed_account: &KeyedAccount,
    amm_context: &AmmContext,
  ) -> Result<Self>
  where
    Self: Sized,
  {
    Ok(HyloJupiter {
      clock: amm_context.clock_ref.clone(),
      state: None,
      exo_entries: Vec::new(),
      exo_account_keys: Vec::new(),
    })
  }

  fn label(&self) -> String {
    "Hylo".to_owned()
  }

  fn program_id(&self) -> Pubkey {
    router::ID
  }

  fn key(&self) -> Pubkey {
    pda::EXO_REGISTRY
  }

  fn get_reserve_mints(&self) -> Vec<Pubkey> {
    ENSHRINED_MINTS
      .into_iter()
      .chain(
        self
          .exo_entries
          .iter()
          .flat_map(|entry| [entry.collateral_mint, entry.levercoin_mint]),
      )
      .collect::<BTreeSet<_>>()
      .into_iter()
      .collect()
  }

  fn get_accounts_to_update(&self) -> Vec<Pubkey> {
    ENSHRINED_ACCOUNTS
      .into_iter()
      .chain(once(pda::EXO_REGISTRY))
      .chain(self.exo_account_keys.iter().copied())
      .collect()
  }

  fn update(&mut self, account_map: &AccountMap) -> Result<()> {
    let entries = exo_entries(account_map)?;
    self.exo_account_keys = exo_pubkeys_from_entries(&entries)?;
    let exo_accounts = self.exo_accounts(account_map, &entries)?;
    self.state = Some(self.build_state(account_map, &exo_accounts)?);
    self.exo_entries = entries;
    Ok(())
  }

  fn quote(&self, params: &QuoteParams) -> Result<Quote> {
    let state = self.state.as_ref().context("state not set")?;
    let quote = state.runtime_quote(
      &self.exo_entries,
      params.input_mint,
      params.output_mint,
      params.amount,
    )?;
    Ok(Quote {
      in_amount: quote.in_amount,
      out_amount: quote.out_amount,
      fee_amount: quote.fee_amount,
      fee_mint: quote.fee_mint,
      fee_pct: fee_pct(quote.fee_amount, quote.fee_base)?,
    })
  }

  fn get_swap_and_account_metas(
    &self,
    p: &SwapParams,
  ) -> Result<SwapAndAccountMetas> {
    let SwapParams {
      source_mint,
      destination_mint,
      token_transfer_authority,
      ..
    } = validate_swap_params(p)?;
    let user = *token_transfer_authority;
    match (*source_mint, *destination_mint) {
      (JITOSOL::MINT, HYUSD::MINT) => {
        Ok(account_metas::mint_stablecoin_lst(user, JITOSOL::MINT))
      }
      (HYLOSOL::MINT, HYUSD::MINT) => {
        Ok(account_metas::mint_stablecoin_lst(user, HYLOSOL::MINT))
      }
      (HYUSD::MINT, JITOSOL::MINT) => {
        Ok(account_metas::redeem_stablecoin_lst(user, JITOSOL::MINT))
      }
      (HYUSD::MINT, HYLOSOL::MINT) => {
        Ok(account_metas::redeem_stablecoin_lst(user, HYLOSOL::MINT))
      }
      (JITOSOL::MINT, XSOL::MINT) => {
        Ok(account_metas::mint_levercoin_lst(user, JITOSOL::MINT))
      }
      (HYLOSOL::MINT, XSOL::MINT) => {
        Ok(account_metas::mint_levercoin_lst(user, HYLOSOL::MINT))
      }
      (XSOL::MINT, JITOSOL::MINT) => {
        Ok(account_metas::redeem_levercoin_lst(user, JITOSOL::MINT))
      }
      (XSOL::MINT, HYLOSOL::MINT) => {
        Ok(account_metas::redeem_levercoin_lst(user, HYLOSOL::MINT))
      }
      (HYUSD::MINT, XSOL::MINT) => {
        Ok(account_metas::convert_stable_to_lever_lst(user))
      }
      (XSOL::MINT, HYUSD::MINT) => {
        Ok(account_metas::convert_lever_to_stable_lst(user))
      }
      (HYUSD::MINT, SHYUSD::MINT) => Ok(account_metas::earn_pool_deposit(user)),
      (SHYUSD::MINT, HYUSD::MINT) => {
        Ok(account_metas::earn_pool_withdraw(user))
      }
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
      (JITOSOL::MINT, USDC::MINT) => Ok(account_metas::swap_lst_to_usdc(
        user,
        JITOSOL::MINT,
        JITOSOL::POOL_STATE,
      )),
      (HYLOSOL::MINT, USDC::MINT) => Ok(account_metas::swap_lst_to_usdc(
        user,
        HYLOSOL::MINT,
        HYLOSOL::POOL_STATE,
      )),
      (USDC::MINT, JITOSOL::MINT) => Ok(account_metas::swap_usdc_to_lst(
        user,
        JITOSOL::MINT,
        JITOSOL::POOL_STATE,
      )),
      (USDC::MINT, HYLOSOL::MINT) => Ok(account_metas::swap_usdc_to_lst(
        user,
        HYLOSOL::MINT,
        HYLOSOL::POOL_STATE,
      )),
      (USDC::MINT, HYUSD::MINT) => {
        Ok(account_metas::mint_stablecoin_usdc(user))
      }
      (HYUSD::MINT, USDC::MINT) => {
        Ok(account_metas::redeem_stablecoin_usdc(user))
      }
      (source, destination) => {
        self.exo_account_metas(user, source, destination)
      }
    }
  }

  fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
    Box::new(self.clone())
  }
}

#[cfg(test)]
mod tests {
  use anchor_lang::pubkey;
  use anyhow::anyhow;
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

  async fn build_jupiter() -> Result<HyloJupiter> {
    let url = std::env::var("RPC_URL")?;
    let client = RpcClient::new(url);
    let key = pda::EXO_REGISTRY;
    let account = client.get_account(&key).await?;
    let jupiter_account = KeyedAccount {
      key,
      account,
      params: None,
    };
    let amm_context = load_amm_context(&client).await?;
    let mut amm =
      HyloJupiter::from_keyed_account(&jupiter_account, &amm_context)?;
    // The first update only discovers the pair account keys from the
    // registry and fails on the pair accounts; the second has them.
    let account_map =
      load_account_map(&client, &amm.get_accounts_to_update()).await?;
    let _ = amm.update(&account_map);
    let account_map =
      load_account_map(&client, &amm.get_accounts_to_update()).await?;
    amm.update(&account_map)?;
    Ok(amm)
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
    let jup = build_jupiter().await?;
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
