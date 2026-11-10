//! State provider trait and implementations
//!
//! Provides abstractions for fetching Hylo protocol state from various sources.

use std::sync::Arc;

use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::{Clock, Pubkey};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use hylo_core::error::CoreError;
use hylo_core::exchange_context::LstExchangeContext;
use hylo_core::pyth::PythOracle;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{Exo, TokenMint, CBBTC, HYPE, ONYC, PST, WETH, ZEC};
use hylo_idl::{pda, with_exo_pairs};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::protocol_state::accounts::{decode, decode_clock};
use crate::protocol_state::{
  build_lst_exchange_context, exo_pair_oracle, exo_pubkeys_from_entries,
  read_exo_entries, ExoPairAccounts, ExoPairState, ProtocolAccounts,
  ProtocolState, EXO_PAIR_ACCOUNTS,
};

/// Trait for fetching protocol state from a data source
#[async_trait]
pub trait StateProvider<C: SolanaClock>: Send + Sync {
  /// Fetch the current protocol state
  ///
  /// # Errors
  /// Returns error if state fetching fails.
  async fn fetch_state(&self) -> Result<ProtocolState<C>>;
}

#[async_trait]
impl<T: StateProvider<C>, C: SolanaClock> StateProvider<C> for Arc<T> {
  async fn fetch_state(&self) -> Result<ProtocolState<C>> {
    (**self).fetch_state().await
  }
}

/// State provider that fetches protocol state via Solana RPC
pub struct RpcStateProvider {
  rpc_client: Arc<RpcClient>,
}

impl RpcStateProvider {
  /// Creates a state provider over an RPC client.
  #[must_use]
  pub fn new(rpc_client: Arc<RpcClient>) -> Self {
    Self { rpc_client }
  }

  /// Fetch the isolated LST exchange context.
  ///
  /// # Errors
  /// Returns error if the fetch or deserialization fails.
  pub async fn fetch_lst_context(&self) -> Result<LstExchangeContext<Clock>> {
    let pubkeys = ProtocolAccounts::lst_pubkeys();
    let data = self
      .rpc_client
      .get_multiple_accounts(&pubkeys)
      .await
      .context("Failed to fetch LST accounts from RPC")?;
    let (hylo, xsol_mint, sol_usd, clock) = match data.as_slice() {
      [Some(hylo), Some(xsol_mint), Some(sol_usd), Some(clock)] => {
        Ok((hylo, xsol_mint, sol_usd, clock))
      }
      _ => Err(anyhow!("Missing LST account")),
    }?;
    build_lst_exchange_context(
      decode_clock(clock)?,
      &decode(hylo, "Hylo")?,
      &decode(xsol_mint, "XSOL mint")?,
      &decode(sol_usd, "SOL/USD Pyth feed")?,
    )
  }

  /// Fetch the isolated pair state for exo collateral `E`.
  ///
  /// # Errors
  /// Returns error if the fetch or deserialization fails.
  pub async fn fetch_exo_pair<E: Exo + PythOracle>(
    &self,
  ) -> Result<ExoPairState<Clock>> {
    let pubkeys = ProtocolAccounts::exo_pubkeys::<E>();
    let data = self
      .rpc_client
      .get_multiple_accounts(&pubkeys)
      .await
      .context("Failed to fetch exo accounts from RPC")?;
    let [entry_accounts @ .., oracle, clock]: [Option<Account>;
      EXO_PAIR_ACCOUNTS + 2] = data
      .try_into()
      .map_err(|_| anyhow!("Missing exo account"))?;
    let raw = ExoPairAccounts::from_fetched(&entry_accounts, oracle)?;
    let clock = clock.as_ref().context("Missing clock account")?;
    raw.pair_state(&E::EXO_ENTRY, decode_clock(clock)?)
  }
}

macro_rules! exo_pair_dispatch {
  ($(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?) => {
    impl RpcStateProvider {
      /// Fetches pair state for a collateral mint known only at runtime.
      ///
      /// # Errors
      /// * [`CoreError::UnknownExoMint`] if the mint backs no exo pair
      /// * Fetch or deserialization failure
      pub async fn fetch_exo_pair_by_mint(
        &self,
        collateral_mint: Pubkey,
      ) -> Result<ExoPairState<Clock>> {
        match collateral_mint {
          $(<$exo>::MINT => self.fetch_exo_pair::<$exo>().await,)+
          _ => Err(CoreError::UnknownExoMint.into()),
        }
      }
    }
  };
}

with_exo_pairs!(exo_pair_dispatch);

#[async_trait]
impl StateProvider<Clock> for RpcStateProvider {
  async fn fetch_state(&self) -> Result<ProtocolState<Clock>> {
    let registry_account = self
      .rpc_client
      .get_account(&pda::EXO_REGISTRY)
      .await
      .context("Failed to fetch exo registry from RPC")?;
    let entries = read_exo_entries(&registry_account.data)?;
    let keys = ProtocolAccounts::PUBKEYS
      .into_iter()
      .chain(exo_pubkeys_from_entries(&entries))
      .collect::<Vec<_>>();
    let account_data = self
      .rpc_client
      .get_multiple_accounts(&keys)
      .await
      .context("Failed to fetch accounts from RPC")?;
    let (base, exo_data) = account_data
      .split_at_checked(ProtocolAccounts::PUBKEYS.len())
      .context("Missing protocol accounts")?;
    let exo_accounts = self.fetch_exo_accounts(exo_data).await?;
    let accounts = ProtocolAccounts::from_fetched(base, exo_accounts)?;
    ProtocolState::try_from(&accounts)
  }
}

impl RpcStateProvider {
  /// Fetches the oracle named by each pair in `exo_data` and assembles the
  /// raw accounts per registry entry.
  async fn fetch_exo_accounts(
    &self,
    exo_data: &[Option<Account>],
  ) -> Result<Vec<ExoPairAccounts>> {
    let entry_accounts = exo_data
      .chunks(EXO_PAIR_ACCOUNTS)
      .map(|chunk| {
        <&[Option<Account>; EXO_PAIR_ACCOUNTS]>::try_from(chunk)
          .map_err(|_| anyhow!("Exo pair accounts missing"))
      })
      .collect::<Result<Vec<_>>>()?;
    let oracle_keys = entry_accounts
      .iter()
      .map(|[exo_pair, ..]| {
        exo_pair_oracle(exo_pair.as_ref().context("Missing exo pair")?)
      })
      .collect::<Result<Vec<_>>>()?;
    let oracles = self
      .rpc_client
      .get_multiple_accounts(&oracle_keys)
      .await
      .context("Failed to fetch exo oracles from RPC")?;
    entry_accounts
      .into_iter()
      .zip(oracles)
      .map(|(accounts, oracle)| ExoPairAccounts::from_fetched(accounts, oracle))
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use fix::prelude::*;
  use hylo_core::exchange_context::ExchangeContext;
  use hylo_core::solana_clock::SolanaClock;
  use solana_rpc_client::nonblocking::rpc_client::RpcClient;

  use super::*;

  fn build_test_rpc_client() -> Arc<RpcClient> {
    // Use RPC_URL env var if set, otherwise default to mainnet
    let rpc_url = std::env::var("RPC_URL")
      .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    Arc::new(RpcClient::new(rpc_url))
  }

  #[tokio::test]
  #[ignore = "requires the exo registry router on mainnet"]
  async fn test_fetch_state() -> Result<()> {
    let rpc_client = build_test_rpc_client();
    let provider = RpcStateProvider::new(rpc_client);
    let state = provider.fetch_state().await?;

    // Verify timestamp is set and matches clock
    assert!(state.fetched_at > 0);
    let clock_timestamp = state.exchange_context.clock.unix_timestamp();
    assert_eq!(state.fetched_at, clock_timestamp);

    // Verify exchange context has valid data
    assert!(state.exchange_context.total_collateral() > UFix64::zero());
    assert!(state
      .exchange_context
      .collateral_ratio()
      .at_least(UFix64::one()));
    assert!(
      state.exchange_context.collateral_usd_price().lower > UFix64::zero()
    );
    assert!(
      state.exchange_context.collateral_usd_price().upper
        >= state.exchange_context.collateral_usd_price().lower
    );

    // Verify mint accounts have valid data
    assert!(state.hyusd_mint.decimals > 0);
    assert!(state.xsol_mint.decimals > 0);
    assert!(state.shyusd_mint.decimals > 0);

    // Verify clock has reasonable values (slot is u64, so just check it's set)
    assert!(state.exchange_context.clock.slot() > 0);
    Ok(())
  }
}
