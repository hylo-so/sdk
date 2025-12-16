//! Quote simulator that uses transaction simulation for accurate compute units

use anchor_client::solana_client::rpc_config::RpcSimulateTransactionConfig;
use anchor_client::solana_sdk::message::v0::Message;
use anchor_client::solana_sdk::message::VersionedMessage;
use anchor_client::solana_sdk::transaction::VersionedTransaction;
use anchor_lang::prelude::{Clock, Pubkey};
use async_trait::async_trait;
use hylo_clients::prelude::{CommitmentConfig, Signature};
use hylo_clients::protocol_state::StateProvider;
use hylo_idl::tokens::TokenMint;

use crate::instruction_builder::InstructionBuilder;
use crate::quote_builder::QuoteBuilder;
use crate::quote_computer::{
  ComputeUnitDefaults, HyloQuoteComputer, QuoteComputer,
};
use crate::quote_strategy::QuoteStrategy;
use crate::rpc::RpcProvider;
use crate::{ComputeUnitMethod, ExecutableQuote};

/// Solana's maximum compute units per transaction
const MAX_COMPUTE_UNITS: u64 = 1_400_000;

/// Simulates transactions to extract compute units
///
/// ## Fallback Behavior
///
/// If simulation fails (e.g., insufficient user balance, RPC errors),
/// gracefully falls back to estimated compute units. This enables quotes
/// for UI previews even when users lack the necessary balance.
///
/// The returned `ExecutableQuote.compute_unit_method` indicates whether
/// values are `Simulated` (accurate via RPC) or `Estimated` (fallback).
pub struct QuoteSimulator<S: StateProvider, R: RpcProvider> {
  builder: QuoteBuilder<S>,
  rpc_provider: R,
}

impl<S: StateProvider, R: RpcProvider> QuoteSimulator<S, R> {
  pub fn new(state_provider: S, rpc_provider: R) -> Self {
    Self {
      builder: QuoteBuilder::new(state_provider),
      rpc_provider,
    }
  }

  /// Build and simulate a quote
  ///
  /// # Errors
  /// Returns error if state fetch, quote computation, instruction building, or
  /// RPC calls fail. Falls back to estimated compute units on simulation
  /// errors.
  pub async fn simulate_quote<IN: TokenMint, OUT: TokenMint>(
    &self,
    amount: u64,
    user_wallet: Pubkey,
    slippage_bps: u16,
  ) -> anyhow::Result<ExecutableQuote>
  where
    HyloQuoteComputer:
      QuoteComputer<IN, OUT, Clock> + ComputeUnitDefaults<IN, OUT, Clock>,
    (): InstructionBuilder<IN, OUT>,
  {
    let built_quote = self
      .builder
      .build_quote::<IN, OUT>(amount, user_wallet, slippage_bps)
      .await?;

    let recent_blockhash = self.rpc_provider.get_latest_blockhash().await?;

    let message = Message::try_compile(
      &user_wallet,
      &built_quote.instructions,
      &[],
      recent_blockhash,
    )?;

    let versioned_tx = VersionedTransaction {
      signatures: vec![Signature::default()],
      message: VersionedMessage::V0(message),
    };

    let (compute_units, compute_units_safe, compute_unit_method) = match self
      .rpc_provider
      .simulate_transaction_with_config(
        versioned_tx,
        RpcSimulateTransactionConfig {
          sig_verify: false,
          replace_recent_blockhash: true,
          commitment: Some(CommitmentConfig::confirmed()),
          ..Default::default()
        },
      )
      .await
    {
      Ok(response) => {
        if response.value.err.is_some() {
          (
            built_quote.compute_units,
            built_quote.compute_units_safe,
            ComputeUnitMethod::Estimated,
          )
        } else if let Some(cu) = response.value.units_consumed {
          if cu > 0 {
            let cu_safe = compute_units_with_safety_buffer(cu);
            (cu, cu_safe, ComputeUnitMethod::Simulated)
          } else {
            (
              built_quote.compute_units,
              built_quote.compute_units_safe,
              ComputeUnitMethod::Estimated,
            )
          }
        } else {
          (
            built_quote.compute_units,
            built_quote.compute_units_safe,
            ComputeUnitMethod::Estimated,
          )
        }
      }
      Err(_err) => (
        built_quote.compute_units,
        built_quote.compute_units_safe,
        ComputeUnitMethod::Estimated,
      ),
    };

    Ok(ExecutableQuote {
      amounts: built_quote.amounts,
      compute_units,
      compute_units_safe,
      instructions: built_quote.instructions,
      compute_unit_method,
    })
  }
}

#[async_trait]
impl<S: StateProvider, R: RpcProvider> QuoteStrategy for QuoteSimulator<S, R> {
  async fn fetch_quote<IN: TokenMint, OUT: TokenMint>(
    &self,
    amount: u64,
    user_wallet: Pubkey,
    slippage_bps: u16,
  ) -> anyhow::Result<ExecutableQuote>
  where
    HyloQuoteComputer:
      QuoteComputer<IN, OUT, Clock> + ComputeUnitDefaults<IN, OUT, Clock>,
    (): InstructionBuilder<IN, OUT>,
  {
    self
      .simulate_quote::<IN, OUT>(amount, user_wallet, slippage_bps)
      .await
  }
}

fn compute_units_with_safety_buffer(compute_units: u64) -> u64 {
  compute_units
    .checked_mul(3)
    .and_then(|x| x.checked_add(1)) // Add 1 before division to round up
    .map_or(MAX_COMPUTE_UNITS, |x| x / 2) // On overflow, use Solana's max
}
