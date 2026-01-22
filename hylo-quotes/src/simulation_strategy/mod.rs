//! Quote strategy using transaction simulation.
//!
//! Builds instructions and simulates transactions to extract output amounts
//! and compute units from emitted events.

mod exchange;
mod stability_pool;

use anchor_lang::prelude::Clock;
use anyhow::Result;
use async_trait::async_trait;
use hylo_clients::prelude::{
  ExchangeClient, ProgramClient, StabilityPoolClient, VersionedTransactionData,
};
use hylo_clients::transaction::{
  BuildTransactionData, RedeemArgs, StabilityPoolArgs, TransactionSyntax,
};
use hylo_clients::util::{
  user_ata_instruction, EXCHANGE_LOOKUP_TABLE, LST, LST_REGISTRY_LOOKUP_TABLE,
  STABILITY_POOL_LOOKUP_TABLE,
};
use hylo_idl::stability_pool::events::UserWithdrawEventV1;
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};

use crate::runtime_quote_strategy::RuntimeQuoteStrategy;
use crate::Local;

pub struct SimulationStrategy {
  pub(crate) exchange_client: ExchangeClient,
  pub(crate) stability_pool_client: StabilityPoolClient,
}

impl SimulationStrategy {
  #[must_use]
  pub fn new(
    exchange_client: ExchangeClient,
    stability_pool_client: StabilityPoolClient,
  ) -> Self {
    Self {
      exchange_client,
      stability_pool_client,
    }
  }
}

#[async_trait]
impl RuntimeQuoteStrategy<Clock> for SimulationStrategy {}

#[async_trait]
impl<L: LST + Local> BuildTransactionData<SHYUSD, L> for SimulationStrategy {
  type Inputs = StabilityPoolArgs;

  async fn build(
    &self,
    StabilityPoolArgs { amount, user }: StabilityPoolArgs,
  ) -> Result<VersionedTransactionData> {
    let withdraw_data = self
      .stability_pool_client
      .build_transaction_data::<SHYUSD, HYUSD>(StabilityPoolArgs {
        amount,
        user,
      })
      .await?;
    let withdraw_tx = self
      .stability_pool_client
      .build_simulation_transaction(&user, &withdraw_data)
      .await?;
    let withdraw_sim = self
      .stability_pool_client
      .simulate_transaction_event::<UserWithdrawEventV1>(&withdraw_tx)
      .await?;

    let mut instructions = vec![user_ata_instruction(&user, &L::MINT)];
    instructions.extend(withdraw_data.instructions);

    if withdraw_sim.stablecoin_withdrawn.bits > 0 {
      let redeem_hyusd = self
        .exchange_client
        .build_transaction_data::<HYUSD, L>(RedeemArgs {
          amount: withdraw_sim.stablecoin_withdrawn.try_into()?,
          user,
          slippage_config: None,
        })
        .await?;
      instructions.push(user_ata_instruction(&user, &HYUSD::MINT));
      instructions.extend(redeem_hyusd.instructions);
    }

    if withdraw_sim.levercoin_withdrawn.bits > 0 {
      let redeem_xsol = self
        .exchange_client
        .build_transaction_data::<XSOL, L>(RedeemArgs {
          amount: withdraw_sim.levercoin_withdrawn.try_into()?,
          user,
          slippage_config: None,
        })
        .await?;
      instructions.push(user_ata_instruction(&user, &XSOL::MINT));
      instructions.extend(redeem_xsol.instructions);
    }

    let lookup_tables = self
      .stability_pool_client
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;

    Ok(VersionedTransactionData::new(instructions, lookup_tables))
  }
}

#[async_trait]
impl TransactionSyntax for SimulationStrategy {}
