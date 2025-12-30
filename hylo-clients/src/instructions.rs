//! Statically type-safe instruction building without requiring client
//! instances.
//!
//! This module provides compile-time type safety for building transaction
//! instructions and determining required lookup tables, without needing RPC
//! calls or client instances.
//!
//! # Example
//!
//! ```rust,no_run
//! use hylo_clients::instructions::InstructionBuilder;
//! use hylo_idl::tokens::{HYUSD, JITOSOL};
//!
//! let instructions = <ExchangeInstructionBuilder as InstructionBuilder<JITOSOL, HYUSD>>::build_instructions(
//!   MintArgs { amount, user, slippage_config },
//! )?;
//! let lookup_tables = ExchangeInstructionBuilder::<JITOSOL, HYUSD>::REQUIRED_LOOKUP_TABLES;
//! ```

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use hylo_idl::exchange::client::args;
use hylo_idl::exchange::instruction_builders::{
  mint_levercoin, mint_stablecoin, redeem_levercoin, redeem_stablecoin,
  swap_lever_to_stable, swap_stable_to_lever,
};
use hylo_idl::stability_pool::client::args as stability_pool_args;
use hylo_idl::stability_pool::instruction_builders::{
  user_deposit, user_withdraw,
};
use hylo_idl::tokens::{TokenMint, HYUSD, SHYUSD, XSOL};

use crate::transaction::{MintArgs, RedeemArgs, StabilityPoolArgs, SwapArgs};
use crate::util::{
  user_ata_instruction, EXCHANGE_LOOKUP_TABLE, LST, LST_REGISTRY_LOOKUP_TABLE,
  STABILITY_POOL_LOOKUP_TABLE,
};

/// Statically type-safe instruction builder for token pair operations.
///
/// # Type Parameters
/// - `IN`: Input token type
/// - `OUT`: Output token type
pub trait InstructionBuilder<IN: TokenMint, OUT: TokenMint> {
  type Inputs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey];

  /// Builds instructions for the token pair operation.
  ///
  /// # Errors
  /// Returns error if instruction building fails.
  fn build_instructions(inputs: Self::Inputs) -> Result<Vec<Instruction>>;
}

/// Instruction builder implementation for exchange operations.
pub struct ExchangeInstructionBuilder;

// ============================================================================
// LST → HYUSD (mint stablecoin)
// ============================================================================

impl<L: LST> InstructionBuilder<L, HYUSD> for ExchangeInstructionBuilder {
  type Inputs = MintArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE];

  fn build_instructions(
    MintArgs {
      amount,
      user,
      slippage_config,
    }: MintArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &HYUSD::MINT);
    let args = args::MintStablecoin {
      amount_lst_to_deposit: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = mint_stablecoin(user, L::MINT, &args);
    Ok(vec![ata, instruction])
  }
}

// ============================================================================
// HYUSD → LST (redeem stablecoin)
// ============================================================================

impl<L: LST> InstructionBuilder<HYUSD, L> for ExchangeInstructionBuilder {
  type Inputs = RedeemArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE];

  fn build_instructions(
    RedeemArgs {
      amount,
      user,
      slippage_config,
    }: RedeemArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &L::MINT);
    let args = args::RedeemStablecoin {
      amount_to_redeem: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = redeem_stablecoin(user, L::MINT, &args);
    Ok(vec![ata, instruction])
  }
}

// ============================================================================
// LST → XSOL (mint levercoin)
// ============================================================================

impl<L: LST> InstructionBuilder<L, XSOL> for ExchangeInstructionBuilder {
  type Inputs = MintArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE];

  fn build_instructions(
    MintArgs {
      amount,
      user,
      slippage_config,
    }: MintArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &XSOL::MINT);
    let args = args::MintLevercoin {
      amount_lst_to_deposit: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = mint_levercoin(user, L::MINT, &args);
    Ok(vec![ata, instruction])
  }
}

// ============================================================================
// XSOL → LST (redeem levercoin)
// ============================================================================

impl<L: LST> InstructionBuilder<XSOL, L> for ExchangeInstructionBuilder {
  type Inputs = RedeemArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE];

  fn build_instructions(
    RedeemArgs {
      amount,
      user,
      slippage_config,
    }: RedeemArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &L::MINT);
    let args = args::RedeemLevercoin {
      amount_to_redeem: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = redeem_levercoin(user, L::MINT, &args);
    Ok(vec![ata, instruction])
  }
}

// ============================================================================
// HYUSD → XSOL (swap stable to lever)
// ============================================================================

impl InstructionBuilder<HYUSD, XSOL> for ExchangeInstructionBuilder {
  type Inputs = SwapArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] = &[EXCHANGE_LOOKUP_TABLE];

  fn build_instructions(
    SwapArgs {
      amount,
      user,
      slippage_config,
    }: SwapArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &XSOL::MINT);
    let args = args::SwapStableToLever {
      amount_stablecoin: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = swap_stable_to_lever(user, &args);
    Ok(vec![ata, instruction])
  }
}

// ============================================================================
// XSOL → HYUSD (swap lever to stable)
// ============================================================================

impl InstructionBuilder<XSOL, HYUSD> for ExchangeInstructionBuilder {
  type Inputs = SwapArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] = &[EXCHANGE_LOOKUP_TABLE];

  fn build_instructions(
    SwapArgs {
      amount,
      user,
      slippage_config,
    }: SwapArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &HYUSD::MINT);
    let args = args::SwapLeverToStable {
      amount_levercoin: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = swap_lever_to_stable(user, &args);
    Ok(vec![ata, instruction])
  }
}

/// Instruction builder implementation for stability pool operations.
pub struct StabilityPoolInstructionBuilder;

// ============================================================================
// HYUSD → SHYUSD (stability pool deposit)
// ============================================================================

impl InstructionBuilder<HYUSD, SHYUSD> for StabilityPoolInstructionBuilder {
  type Inputs = StabilityPoolArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE, STABILITY_POOL_LOOKUP_TABLE];

  fn build_instructions(
    StabilityPoolArgs { amount, user }: StabilityPoolArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &SHYUSD::MINT);
    let args = stability_pool_args::UserDeposit {
      amount_stablecoin: amount.bits,
    };

    let instruction = user_deposit(user, &args);

    Ok(vec![ata, instruction])
  }
}

// ============================================================================
// SHYUSD → HYUSD (stability pool withdrawal)
// ============================================================================

impl InstructionBuilder<SHYUSD, HYUSD> for StabilityPoolInstructionBuilder {
  type Inputs = StabilityPoolArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE, STABILITY_POOL_LOOKUP_TABLE];

  fn build_instructions(
    StabilityPoolArgs { amount, user }: StabilityPoolArgs,
  ) -> Result<Vec<Instruction>> {
    let hyusd_ata = user_ata_instruction(&user, &HYUSD::MINT);
    let xsol_ata = user_ata_instruction(&user, &XSOL::MINT);
    let args = stability_pool_args::UserWithdraw {
      amount_lp_token: amount.bits,
    };
    let instruction = user_withdraw(user, &args);
    Ok(vec![hyusd_ata, xsol_ata, instruction])
  }
}
