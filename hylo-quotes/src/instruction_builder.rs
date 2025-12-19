//! Type-safe instruction builders
//!
//! Uses type-safe generics matching the SDK pattern to ensure compile-time
//! correctness. Each `<IN, OUT>` token pair has a dedicated implementation that
//! uses hardcoded mint addresses matching the type parameters, preventing
//! invalid instruction construction.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use hylo_clients::prelude::{UFix64, N4, N6, N9};
use hylo_core::idl::exchange::client::args as ex_args;
use hylo_core::idl::stability_pool::client::args as sp_args;
use hylo_core::slippage_config::SlippageConfig;
use hylo_idl::exchange::instruction_builders::{
  mint_levercoin, mint_stablecoin, redeem_levercoin, redeem_stablecoin,
  swap_lever_to_stable, swap_stable_to_lever,
};
use hylo_idl::stability_pool::instruction_builders::user_deposit;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::{QuoteAmounts, SupportedPair};

/// Trait for building instructions for token pair operations.
///
/// Each `<IN, OUT>` combination has a dedicated implementation, enabling
/// type-safe dispatch at compile time.
pub trait InstructionBuilder<IN: TokenMint, OUT: TokenMint>
where
  (IN, OUT): SupportedPair<IN, OUT>,
{
  /// Build instructions for the token pair operation.
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction>;
}

/// Zero-sized type used for instruction builder implementations.
pub struct HyloInstructionBuilder;

/// Build instructions for a token pair operation.
pub(crate) fn build_instructions<IN: TokenMint, OUT: TokenMint>(
  quote: &QuoteAmounts,
  user: Pubkey,
  slippage_bps: u16,
) -> Vec<Instruction>
where
  (IN, OUT): SupportedPair<IN, OUT>,
  HyloInstructionBuilder: InstructionBuilder<IN, OUT>,
{
  HyloInstructionBuilder::build(quote, user, slippage_bps)
}

// ============================================================================
// Implementations for JITOSOL → HYUSD (mint stablecoin)
// ============================================================================

impl InstructionBuilder<JITOSOL, HYUSD> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    // Build args from quote
    let args = ex_args::MintStablecoin {
      amount_lst_to_deposit: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    // Use existing builder instead of duplicating account construction
    let mint_ix = mint_stablecoin(user, JITOSOL::MINT, &args);

    let create_ata_ix = create_ata_instruction(user, HYUSD::MINT);

    vec![create_ata_ix, mint_ix]
  }
}

// ============================================================================
// Implementations for HYUSD → JITOSOL (redeem stablecoin)
// ============================================================================

impl InstructionBuilder<HYUSD, JITOSOL> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N9>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::RedeemStablecoin {
      amount_to_redeem: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let redeem_ix = redeem_stablecoin(user, JITOSOL::MINT, &args);

    let create_ata_ix = create_ata_instruction(user, JITOSOL::MINT);

    vec![create_ata_ix, redeem_ix]
  }
}

// ============================================================================
// Implementations for HYLOSOL → HYUSD (mint stablecoin)
// ============================================================================

impl InstructionBuilder<HYLOSOL, HYUSD> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::MintStablecoin {
      amount_lst_to_deposit: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let mint_ix = mint_stablecoin(user, HYLOSOL::MINT, &args);

    let create_ata_ix = create_ata_instruction(user, HYUSD::MINT);

    vec![create_ata_ix, mint_ix]
  }
}

// ============================================================================
// Implementations for HYUSD → HYLOSOL (redeem stablecoin)
// ============================================================================

impl InstructionBuilder<HYUSD, HYLOSOL> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N9>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::RedeemStablecoin {
      amount_to_redeem: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let redeem_ix = redeem_stablecoin(user, HYLOSOL::MINT, &args);

    let create_ata_ix = create_ata_instruction(user, HYLOSOL::MINT);

    vec![create_ata_ix, redeem_ix]
  }
}

impl InstructionBuilder<JITOSOL, XSOL> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::MintLevercoin {
      amount_lst_to_deposit: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let mint_ix = mint_levercoin(user, JITOSOL::MINT, &args);

    let create_ata_ix = create_ata_instruction(user, XSOL::MINT);

    vec![create_ata_ix, mint_ix]
  }
}

impl InstructionBuilder<XSOL, JITOSOL> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N9>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::RedeemLevercoin {
      amount_to_redeem: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let redeem_ix = redeem_levercoin(user, JITOSOL::MINT, &args);

    let create_ata_ix = create_ata_instruction(user, JITOSOL::MINT);

    vec![create_ata_ix, redeem_ix]
  }
}

impl InstructionBuilder<HYLOSOL, XSOL> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::MintLevercoin {
      amount_lst_to_deposit: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let mint_ix = mint_levercoin(user, HYLOSOL::MINT, &args);

    let create_ata_ix = create_ata_instruction(user, XSOL::MINT);

    vec![create_ata_ix, mint_ix]
  }
}

impl InstructionBuilder<XSOL, HYLOSOL> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N9>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::RedeemLevercoin {
      amount_to_redeem: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let redeem_ix = redeem_levercoin(user, HYLOSOL::MINT, &args);

    let create_ata_ix = create_ata_instruction(user, HYLOSOL::MINT);

    vec![create_ata_ix, redeem_ix]
  }
}

impl InstructionBuilder<HYUSD, XSOL> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::SwapStableToLever {
      amount_stablecoin: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let swap_ix = swap_stable_to_lever(user, &args);

    let create_ata_ix = create_ata_instruction(user, XSOL::MINT);

    vec![create_ata_ix, swap_ix]
  }
}

impl InstructionBuilder<XSOL, HYUSD> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let args = ex_args::SwapLeverToStable {
      amount_levercoin: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let swap_ix = swap_lever_to_stable(user, &args);

    let create_ata_ix = create_ata_instruction(user, HYUSD::MINT);

    vec![create_ata_ix, swap_ix]
  }
}

impl InstructionBuilder<HYUSD, SHYUSD> for HyloInstructionBuilder {
  fn build(
    quote: &QuoteAmounts,
    user: Pubkey,
    _slippage_bps: u16,
  ) -> Vec<Instruction> {
    let args = sp_args::UserDeposit {
      amount_stablecoin: quote.amount_in,
    };

    let deposit_ix = user_deposit(user, &args);

    let create_ata_ix = create_ata_instruction(user, SHYUSD::MINT);

    vec![create_ata_ix, deposit_ix]
  }
}

/// Helper to create Associated Token Account instruction
///
/// Uses the idempotent create instruction - will succeed even if ATA already
/// exists
fn create_ata_instruction(owner: Pubkey, mint: Pubkey) -> Instruction {
  create_associated_token_account_idempotent(
    &owner, // payer
    &owner, // owner
    &mint,  // mint
    &anchor_spl::token::ID,
  )
}
