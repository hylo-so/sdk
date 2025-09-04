#![allow(clippy::upper_case_acronyms)]

use anchor_client::solana_sdk::signature::Signature;
use anchor_lang::prelude::Pubkey;
use anchor_lang::{AnchorDeserialize, Discriminator};
use anyhow::Result;
use fix::prelude::*;
use hylo_idl::exchange::types::SlippageConfig;
pub use hylo_idl::tokens::{HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::util::REFERENCE_WALLET;

/// Price oracle for Hylo token pairs.
#[async_trait::async_trait]
pub trait SimulatePrice<I, O>:
  BuildTransactionData<I, O> + ProgramClient
where
  <Self as BuildTransactionData<I, O>>::Inputs: QuoteInput,
{
  type OutExp;
  type Event: AnchorDeserialize + Discriminator;

  fn from_event(e: &Self::Event) -> Result<UFix64<Self::OutExp>>;

  /// Gets price quote for 1 unit of input token to output token.
  async fn simulate(&self) -> Result<UFix64<Self::OutExp>> {
    let args = self
      .build(<Self as BuildTransactionData<I, O>>::Inputs::quote_input(
        REFERENCE_WALLET,
      ))
      .await?;
    let tx = self
      .build_simulation_transaction(&REFERENCE_WALLET, &args)
      .await?;
    let event = self.simulate_transaction_event::<Self::Event>(&tx).await?;
    Self::from_event(&event)
  }
}

#[async_trait::async_trait]
pub trait SimulatePriceWithEnv<I, O>
where
  Self: BuildTransactionData<I, O>,
{
  type OutExp;
  type Env: Send;
  async fn simulate_with_env(
    &self,
    env: Self::Env,
  ) -> Result<UFix64<Self::OutExp>>;
}

pub trait QuoteInput {
  fn quote_input(user: Pubkey) -> Self;
}

pub struct MintArgs {
  pub amount: UFix64<N9>,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

impl QuoteInput for MintArgs {
  fn quote_input(user: Pubkey) -> Self {
    MintArgs {
      amount: UFix64::one(),
      user,
      slippage_config: None,
    }
  }
}

pub struct RedeemArgs {
  pub amount: UFix64<N6>,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

impl QuoteInput for RedeemArgs {
  fn quote_input(user: Pubkey) -> Self {
    RedeemArgs {
      amount: UFix64::one(),
      user,
      slippage_config: None,
    }
  }
}

pub struct SwapArgs {
  pub amount: UFix64<N6>,
  pub user: Pubkey,
}

impl QuoteInput for SwapArgs {
  fn quote_input(user: Pubkey) -> Self {
    SwapArgs {
      amount: UFix64::one(),
      user,
    }
  }
}

pub struct StabilityPoolArgs {
  pub amount: UFix64<N6>,
  pub user: Pubkey,
}

impl QuoteInput for StabilityPoolArgs {
  fn quote_input(user: Pubkey) -> Self {
    StabilityPoolArgs {
      amount: UFix64::one(),
      user,
    }
  }
}

#[async_trait::async_trait]
pub trait BuildTransactionData<I, O> {
  type Inputs: Send + Sync + 'static;

  async fn build(
    &self,
    inputs: Self::Inputs,
  ) -> Result<VersionedTransactionData>;
}

#[async_trait::async_trait]
pub trait TransactionSyntax {
  async fn run_transaction<I, O>(
    &self,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<Signature>
  where
    Self: BuildTransactionData<I, O> + ProgramClient,
  {
    let args = self.build(inputs).await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }

  async fn build_transaction_data<I, O>(
    &self,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<VersionedTransactionData>
  where
    Self: BuildTransactionData<I, O>,
  {
    self.build(inputs).await
  }

  async fn quote<I, O>(
    &self,
  ) -> Result<UFix64<<Self as SimulatePrice<I, O>>::OutExp>>
  where
    Self: SimulatePrice<I, O>,
    <Self as BuildTransactionData<I, O>>::Inputs: QuoteInput,
  {
    self.simulate().await
  }

  async fn quote_with_env<I, O>(
    &self,
    env: <Self as SimulatePriceWithEnv<I, O>>::Env,
  ) -> Result<UFix64<<Self as SimulatePriceWithEnv<I, O>>::OutExp>>
  where
    Self: SimulatePriceWithEnv<I, O>,
  {
    self.simulate_with_env(env).await
  }
}
