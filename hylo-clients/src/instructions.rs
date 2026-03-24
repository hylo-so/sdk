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
//! use hylo_clients::instructions::{ExchangeInstructionBuilder, InstructionBuilder};
//! use hylo_clients::transaction::MintArgs;
//! use hylo_clients::prelude::*;
//! use hylo_idl::tokens::{HYUSD, JITOSOL};
//!
//! # fn main() -> anyhow::Result<()> {
//! let amount = UFix64::<N9>::one();
//! let user = Pubkey::new_unique();
//! let slippage_config = None;
//!
//! let instructions = ExchangeInstructionBuilder::build_instructions::<JITOSOL, HYUSD>(
//!   MintArgs { amount, user, slippage_config },
//! )?;
//! let lookup_tables = ExchangeInstructionBuilder::lookup_tables::<JITOSOL, HYUSD>();
//! # Ok(())
//! # }
//! ```

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use hylo_idl::exchange::account_builders;
use hylo_idl::exchange::client::args as exchange_args;
use hylo_idl::exchange::instruction_builders::{
  convert_lever_to_stable_lst, convert_stable_to_lever_lst, mint_levercoin_lst,
  mint_stablecoin_lst, redeem_levercoin_lst, redeem_stablecoin_lst,
  swap_lst_to_lst,
};
use hylo_idl::pda;
use hylo_idl::router::client::args as router_args;
use hylo_idl::router::instruction_builders::route;
use hylo_idl::stability_pool::client::args as stability_pool_args;
use hylo_idl::stability_pool::instruction_builders::{
  user_deposit, user_withdraw,
};
use hylo_idl::tokens::{TokenMint, CBBTC, HYUSD, SHYUSD, USDC, XBTC, XSOL};

use crate::transaction::{
  LstSwapArgs, MintArgs, RedeemArgs, RouterArgs, StabilityPoolArgs, SwapArgs,
};
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
  fn build(inputs: Self::Inputs) -> Result<Vec<Instruction>>;
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

  fn build(
    MintArgs {
      amount,
      user,
      slippage_config,
    }: MintArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &HYUSD::MINT);
    let args = exchange_args::MintStablecoinLst {
      amount_lst_to_deposit: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = mint_stablecoin_lst(user, L::MINT, &args);
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

  fn build(
    RedeemArgs {
      amount,
      user,
      slippage_config,
    }: RedeemArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &L::MINT);
    let args = exchange_args::RedeemStablecoinLst {
      amount_to_redeem: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = redeem_stablecoin_lst(user, L::MINT, &args);
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

  fn build(
    MintArgs {
      amount,
      user,
      slippage_config,
    }: MintArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &XSOL::MINT);
    let args = exchange_args::MintLevercoinLst {
      amount_lst_to_deposit: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = mint_levercoin_lst(user, L::MINT, &args);
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

  fn build(
    RedeemArgs {
      amount,
      user,
      slippage_config,
    }: RedeemArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &L::MINT);
    let args = exchange_args::RedeemLevercoinLst {
      amount_to_redeem: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = redeem_levercoin_lst(user, L::MINT, &args);
    Ok(vec![ata, instruction])
  }
}

// ============================================================================
// HYUSD → XSOL (convert stable to lever)
// ============================================================================

impl InstructionBuilder<HYUSD, XSOL> for ExchangeInstructionBuilder {
  type Inputs = SwapArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] = &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    SwapArgs {
      amount,
      user,
      slippage_config,
    }: SwapArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &XSOL::MINT);
    let args = exchange_args::ConvertStableToLeverLst {
      amount_stablecoin: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = convert_stable_to_lever_lst(user, &args);
    Ok(vec![ata, instruction])
  }
}

// ============================================================================
// XSOL → HYUSD (convert lever to stable)
// ============================================================================

impl InstructionBuilder<XSOL, HYUSD> for ExchangeInstructionBuilder {
  type Inputs = SwapArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] = &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    SwapArgs {
      amount,
      user,
      slippage_config,
    }: SwapArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &HYUSD::MINT);
    let args = exchange_args::ConvertLeverToStableLst {
      amount_levercoin: amount.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = convert_lever_to_stable_lst(user, &args);
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

  fn build(
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

  fn build(
    StabilityPoolArgs { amount, user }: StabilityPoolArgs,
  ) -> Result<Vec<Instruction>> {
    let hyusd_ata = user_ata_instruction(&user, &HYUSD::MINT);
    let args = stability_pool_args::UserWithdraw {
      amount_lp_token: amount.bits,
    };
    let instruction = user_withdraw(user, &args);
    Ok(vec![hyusd_ata, instruction])
  }
}

// ============================================================================
// LST → LST (swap within exchange)
// ============================================================================

impl<L1: LST, L2: LST> InstructionBuilder<L1, L2>
  for ExchangeInstructionBuilder
{
  type Inputs = LstSwapArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE];

  fn build(
    LstSwapArgs {
      amount_lst_a,
      lst_a_mint,
      lst_b_mint,
      user,
      slippage_config,
    }: LstSwapArgs,
  ) -> Result<Vec<Instruction>> {
    let user_lst_b_ata = user_ata_instruction(&user, &L2::MINT);
    let args = exchange_args::SwapLstToLst {
      amount_lst_a: amount_lst_a.bits,
      slippage_config: slippage_config.map(Into::into),
    };
    let instruction = swap_lst_to_lst(user, lst_a_mint, lst_b_mint, &args);
    Ok(vec![user_lst_b_ata, instruction])
  }
}

// ============================================================================
// Router-based instruction builder for exo/USDC operations
// ============================================================================

/// Instruction builder for exo and USDC operations routed through
/// the Hylo router program.
pub struct RouterInstructionBuilder;

/// Builds a router `Route` instruction wrapping exchange accounts.
fn build_route_instruction<A: anchor_lang::ToAccountMetas>(
  token_a: Pubkey,
  token_b: Pubkey,
  amount: u64,
  slippage_config: Option<hylo_core::slippage_config::SlippageConfig>,
  inner_accounts: &A,
) -> Instruction {
  let args = router_args::Route {
    token_a,
    token_b,
    amount,
    slippage_config: slippage_config.map(Into::into),
  };
  route(&args, inner_accounts)
}

// USDC -> HYUSD (mint stablecoin from USDC)
impl InstructionBuilder<USDC, HYUSD> for RouterInstructionBuilder {
  type Inputs = RouterArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    RouterArgs {
      amount,
      user,
      slippage_config,
    }: RouterArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &HYUSD::MINT);
    let accounts = account_builders::mint_stablecoin_usdc(user);
    let instruction = build_route_instruction(
      USDC::MINT,
      HYUSD::MINT,
      amount,
      slippage_config,
      &accounts,
    );
    Ok(vec![ata, instruction])
  }
}

// HYUSD -> USDC (redeem stablecoin to USDC)
impl InstructionBuilder<HYUSD, USDC> for RouterInstructionBuilder {
  type Inputs = RouterArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    RouterArgs {
      amount,
      user,
      slippage_config,
    }: RouterArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &USDC::MINT);
    let accounts = account_builders::redeem_stablecoin_usdc(user);
    let instruction = build_route_instruction(
      HYUSD::MINT,
      USDC::MINT,
      amount,
      slippage_config,
      &accounts,
    );
    Ok(vec![ata, instruction])
  }
}

// CBBTC -> HYUSD (mint stablecoin from exo collateral)
impl InstructionBuilder<CBBTC, HYUSD> for RouterInstructionBuilder {
  type Inputs = RouterArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    RouterArgs {
      amount,
      user,
      slippage_config,
    }: RouterArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &HYUSD::MINT);
    let accounts = account_builders::mint_stablecoin_exo(
      user,
      CBBTC::MINT,
      pda::BTC_USD_PYTH_FEED,
    );
    let instruction = build_route_instruction(
      CBBTC::MINT,
      HYUSD::MINT,
      amount,
      slippage_config,
      &accounts,
    );
    Ok(vec![ata, instruction])
  }
}

// HYUSD -> CBBTC (redeem stablecoin for exo collateral)
impl InstructionBuilder<HYUSD, CBBTC> for RouterInstructionBuilder {
  type Inputs = RouterArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    RouterArgs {
      amount,
      user,
      slippage_config,
    }: RouterArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &CBBTC::MINT);
    let accounts = account_builders::redeem_stablecoin_exo(
      user,
      CBBTC::MINT,
      pda::BTC_USD_PYTH_FEED,
    );
    let instruction = build_route_instruction(
      HYUSD::MINT,
      CBBTC::MINT,
      amount,
      slippage_config,
      &accounts,
    );
    Ok(vec![ata, instruction])
  }
}

// CBBTC -> XBTC (mint levercoin from exo collateral)
impl InstructionBuilder<CBBTC, XBTC> for RouterInstructionBuilder {
  type Inputs = RouterArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    RouterArgs {
      amount,
      user,
      slippage_config,
    }: RouterArgs,
  ) -> Result<Vec<Instruction>> {
    let levercoin_mint = pda::exo_levercoin_mint(CBBTC::MINT);
    let ata = user_ata_instruction(&user, &levercoin_mint);
    let accounts = account_builders::mint_levercoin_exo(
      user,
      CBBTC::MINT,
      pda::BTC_USD_PYTH_FEED,
    );
    let instruction = build_route_instruction(
      CBBTC::MINT,
      XBTC::MINT,
      amount,
      slippage_config,
      &accounts,
    );
    Ok(vec![ata, instruction])
  }
}

// XBTC -> CBBTC (redeem levercoin for exo collateral)
impl InstructionBuilder<XBTC, CBBTC> for RouterInstructionBuilder {
  type Inputs = RouterArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    RouterArgs {
      amount,
      user,
      slippage_config,
    }: RouterArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &CBBTC::MINT);
    let accounts = account_builders::redeem_levercoin_exo(
      user,
      CBBTC::MINT,
      pda::BTC_USD_PYTH_FEED,
    );
    let instruction = build_route_instruction(
      XBTC::MINT,
      CBBTC::MINT,
      amount,
      slippage_config,
      &accounts,
    );
    Ok(vec![ata, instruction])
  }
}

// HYUSD -> XBTC (swap stable to exo lever)
impl InstructionBuilder<HYUSD, XBTC> for RouterInstructionBuilder {
  type Inputs = RouterArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    RouterArgs {
      amount,
      user,
      slippage_config,
    }: RouterArgs,
  ) -> Result<Vec<Instruction>> {
    let levercoin_mint = pda::exo_levercoin_mint(CBBTC::MINT);
    let ata = user_ata_instruction(&user, &levercoin_mint);
    let accounts = account_builders::convert_stable_to_lever_exo(
      user,
      CBBTC::MINT,
      pda::BTC_USD_PYTH_FEED,
    );
    let instruction = build_route_instruction(
      HYUSD::MINT,
      XBTC::MINT,
      amount,
      slippage_config,
      &accounts,
    );
    Ok(vec![ata, instruction])
  }
}

// XBTC -> HYUSD (swap exo lever to stable)
impl InstructionBuilder<XBTC, HYUSD> for RouterInstructionBuilder {
  type Inputs = RouterArgs;

  const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] =
    &[EXCHANGE_LOOKUP_TABLE];

  fn build(
    RouterArgs {
      amount,
      user,
      slippage_config,
    }: RouterArgs,
  ) -> Result<Vec<Instruction>> {
    let ata = user_ata_instruction(&user, &HYUSD::MINT);
    let accounts = account_builders::convert_lever_to_stable_exo(
      user,
      CBBTC::MINT,
      pda::BTC_USD_PYTH_FEED,
    );
    let instruction = build_route_instruction(
      XBTC::MINT,
      HYUSD::MINT,
      amount,
      slippage_config,
      &accounts,
    );
    Ok(vec![ata, instruction])
  }
}
