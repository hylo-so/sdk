//! Production account layout regression for the optional USDC oracle lane.
//! The snapshot is copied unchanged from Mio's September 30 production fixture.

#![cfg(not(feature = "shadow"))]

use std::collections::BTreeMap;

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::Clock;
use anchor_lang::{AccountDeserialize, AccountSerialize};
use fix::prelude::*;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::ExchangeContext;
use hylo_idl::tokens::{HYUSD, JITOSOL, USDC, XSOL};
use hylo_quotes::prelude::{
  ProtocolAccounts, ProtocolState, TokenOperationExt,
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

fn accounts() -> ProtocolAccounts {
  let accounts: BTreeMap<String, Account> =
    serde_json::from_str(include_str!("data/production-2026-09-30.json"))
      .unwrap();
  let get = |key: &str| accounts[key].clone();
  ProtocolAccounts {
    hylo: get("hylo"),
    jitosol_header: get("jitosol_header"),
    hylosol_header: get("hylosol_header"),
    hyusd_mint: get("hyusd_mint"),
    shyusd_mint: get("shyusd_mint"),
    xsol_mint: get("xsol_mint"),
    pool_config: get("pool_config"),
    hyusd_pool: get("hyusd_pool"),
    sol_usd_pyth: get("sol_usd_pyth"),
    clock: get("clock"),
    cbbtc_exo_pair: get("cbbtc_exo_pair"),
    cbbtc_vault: get("cbbtc_vault"),
    xbtc_mint: get("xbtc_mint"),
    btc_usd_pyth: get("btc_usd_pyth"),
    usdc_pair: get("usdc_pair"),
    usdc_usd_pyth: get("usdc_usd_pyth"),
    jitosol_pool_state: get("jitosol_pool_state"),
    hylosol_pool_state: get("hylosol_pool_state"),
    jitosol_vault: get("jitosol_vault"),
    hylosol_vault: get("hylosol_vault"),
    usdc_vault: get("usdc_vault"),
    hype_pair_accounts: None,
    onyc_pair_accounts: None,
    pst_pair_accounts: None,
    weth_pair_accounts: None,
    zec_pair_accounts: None,
  }
}

fn with_oracle(
  accounts: ProtocolAccounts,
  oracle: &PriceUpdateV2,
) -> ProtocolAccounts {
  let mut data = Vec::new();
  oracle.try_serialize(&mut data).unwrap();
  ProtocolAccounts {
    usdc_usd_pyth: Account {
      data,
      ..accounts.usdc_usd_pyth
    },
    ..accounts
  }
}

fn usdc_oracle(accounts: &ProtocolAccounts) -> PriceUpdateV2 {
  PriceUpdateV2::try_deserialize(&mut accounts.usdc_usd_pyth.data.as_slice())
    .unwrap()
}

#[test]
fn wide_confidence_preserves_independent_redemptions_and_rejects_usdc() {
  let accounts = accounts();
  let healthy = ProtocolState::<Clock>::try_from(&accounts).unwrap();
  let hyusd = healthy
    .output::<HYUSD, JITOSOL>(UFix64::new(1_000_000))
    .unwrap();
  let xsol = healthy
    .output::<XSOL, JITOSOL>(UFix64::new(1_000_000))
    .unwrap();
  let stable_nav = healthy.exchange_context.stablecoin_nav().unwrap();
  let lever_nav = healthy.exchange_context.levercoin_redeem_nav().unwrap();
  let oracle = usdc_oracle(&accounts);
  let wide = PriceUpdateV2 {
    price_message: pyth_solana_receiver_sdk::price_update::PriceFeedMessage {
      conf: oracle.price_message.price.unsigned_abs(),
      ..oracle.price_message
    },
    ..oracle
  };
  let degraded =
    ProtocolState::<Clock>::try_from(&with_oracle(accounts, &wide)).unwrap();
  assert_eq!(
    degraded.usdc_exchange_state.usdc_usd_oracle,
    Err(CoreError::PythOracleConfidence)
  );
  assert_eq!(
    degraded
      .output::<HYUSD, JITOSOL>(UFix64::new(1_000_000))
      .unwrap(),
    hyusd
  );
  assert_eq!(
    degraded
      .output::<XSOL, JITOSOL>(UFix64::new(1_000_000))
      .unwrap(),
    xsol
  );
  assert_eq!(
    degraded.exchange_context.stablecoin_nav().unwrap(),
    stable_nav
  );
  assert_eq!(
    degraded.exchange_context.levercoin_redeem_nav().unwrap(),
    lever_nav
  );
  assert!(matches!(
    degraded.output::<USDC, HYUSD>(UFix64::new(1_000_000)),
    Err(CoreError::PythOracleConfidence)
  ));
  assert!(matches!(
    degraded.output::<HYUSD, USDC>(UFix64::new(1_000_000)),
    Err(CoreError::PythOracleConfidence)
  ));
}

#[test]
fn stale_usdc_preserves_its_error_for_both_routes() {
  let accounts = accounts();
  let oracle = usdc_oracle(&accounts);
  let outdated = PriceUpdateV2 {
    price_message: pyth_solana_receiver_sdk::price_update::PriceFeedMessage {
      publish_time: 1,
      ..oracle.price_message
    },
    ..oracle
  };
  let state =
    ProtocolState::<Clock>::try_from(&with_oracle(accounts, &outdated))
      .unwrap();
  assert!(matches!(
    state.output::<USDC, HYUSD>(UFix64::new(1_000_000)),
    Err(CoreError::PythOracleOutdated)
  ));
  assert!(matches!(
    state.output::<HYUSD, USDC>(UFix64::new(1_000_000)),
    Err(CoreError::PythOracleOutdated)
  ));
}

#[test]
fn malformed_usdc_accounts_are_fatal() {
  let accounts = accounts();
  for malformed in [
    ProtocolAccounts {
      usdc_pair: Account::default(),
      ..accounts.clone()
    },
    ProtocolAccounts {
      usdc_usd_pyth: Account::default(),
      ..accounts.clone()
    },
    ProtocolAccounts {
      usdc_vault: Account::default(),
      ..accounts
    },
  ] {
    assert!(ProtocolState::<Clock>::try_from(&malformed).is_err());
  }
}
