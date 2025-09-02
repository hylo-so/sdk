#![allow(clippy::upper_case_acronyms)]

use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::util::REFERENCE_WALLET;

use anchor_client::solana_sdk::signature::Signature;
use anchor_lang::prelude::Pubkey;
use anchor_lang::{AnchorDeserialize, Discriminator};
use anyhow::Result;
use fix::prelude::*;
use hylo_idl::exchange::types::SlippageConfig;
use hylo_idl::pda;

pub trait Mint {
  const MINT: Pubkey;
}

pub struct XSOL;

impl Mint for XSOL {
  const MINT: Pubkey = pda::XSOL;
}

pub struct HYUSD;

impl Mint for HYUSD {
  const MINT: Pubkey = pda::HYUSD;
}

pub struct SHYUSD;

impl Mint for SHYUSD {
  const MINT: Pubkey = pda::SHYUSD;
}

pub struct JITOSOL;

impl Mint for JITOSOL {
  const MINT: Pubkey = pda::JITOSOL;
}

/// Price oracle for Hylo token pairs.
#[async_trait::async_trait]
pub trait SimulatePrice<I, O>:
  BuildTransactionData<I, O> + ProgramClient
{
  type OutExp;
  type Event: AnchorDeserialize + Discriminator;

  fn from_event(e: &Self::Event) -> UFix64<Self::OutExp>;

  /// Gets price quote for 1 unit of input token to output token.
  async fn simulate(&self) -> Result<UFix64<Self::OutExp>> {
    let args = self.build(self.quote_inputs(REFERENCE_WALLET)).await?;
    let tx = self
      .build_simulation_transaction(&REFERENCE_WALLET, &args)
      .await?;
    let event = self.simulate_transaction_event::<Self::Event>(&tx).await?;
    Ok(Self::from_event(&event))
  }
}

pub struct MintArgs {
  pub amount: UFix64<N9>,
  pub lst_mint: Pubkey,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

pub struct RedeemArgs {
  pub amount: UFix64<N6>,
  pub lst_mint: Pubkey,
  pub user: Pubkey,
  pub slippage_config: Option<SlippageConfig>,
}

pub struct SwapArgs {
  pub amount: UFix64<N6>,
  pub user: Pubkey,
}

#[async_trait::async_trait]
pub trait BuildTransactionData<I, O> {
  type Inputs: Send + Sync + 'static;

  fn quote_inputs(&self, user: Pubkey) -> Self::Inputs;

  async fn build(
    &self,
    inputs: Self::Inputs,
  ) -> Result<VersionedTransactionData>;
}

#[async_trait::async_trait]
pub trait RunTransaction<I, O>:
  BuildTransactionData<I, O> + ProgramClient
{
  async fn run(
    &self,
    inputs: <Self as BuildTransactionData<I, O>>::Inputs,
  ) -> Result<Signature> {
    let args = self.build(inputs).await?;
    let sig = self.send_v0_transaction(&args).await?;
    Ok(sig)
  }
}
