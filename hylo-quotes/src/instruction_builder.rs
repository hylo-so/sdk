//! Type-safe instruction builders
//!
//! Uses type-safe generics matching the SDK pattern to ensure compile-time
//! correctness. Each `<IN, OUT>` token pair has a dedicated implementation that
//! uses hardcoded mint addresses matching the type parameters, preventing
//! invalid instruction construction.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{system_program, InstructionData, ToAccountMetas};
use anchor_spl::associated_token::spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use hylo_clients::prelude::{UFix64, N4, N6, N9};
use hylo_core::idl::exchange::client::{
  accounts as ex_accounts, args as ex_args,
};
use hylo_core::idl::stability_pool::client::{
  accounts as sp_accounts, args as sp_args,
};
use hylo_core::idl::{ata, exchange, pda};
use hylo_core::pyth::SOL_USD_PYTH_FEED;
use hylo_core::slippage_config::SlippageConfig;
use hylo_idl::stability_pool;
use hylo_idl::tokens::{TokenMint, HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL};

use crate::QuoteAmounts;

/// Type-safe instruction builder trait (SDK pattern)
///
/// The trait is parameterized by token types - each `<IN, OUT>` combination
/// is a separate trait, allowing multiple impl blocks.
pub trait InstructionBuilder<IN: TokenMint, OUT: TokenMint>:
  Send + Sync
{
  /// Build instructions for token pair operation
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction>;
}

// ============================================================================
// Implementations for JITOSOL → HYUSD (mint stablecoin)
// ============================================================================

impl InstructionBuilder<JITOSOL, HYUSD> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::MintStablecoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(JITOSOL::MINT),
      vault_auth: pda::vault_auth(JITOSOL::MINT),
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_vault: pda::fee_vault(JITOSOL::MINT),
      lst_vault: pda::vault(JITOSOL::MINT),
      lst_header: pda::lst_header(JITOSOL::MINT),
      user_lst_ta: ata!(user, JITOSOL::MINT),
      user_stablecoin_ta: pda::hyusd_ata(user),
      lst_mint: JITOSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::MintStablecoin {
      amount_lst_to_deposit: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let mint_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, HYUSD::MINT);

    vec![create_ata_ix, mint_ix]
  }
}

// ============================================================================
// Implementations for HYUSD → JITOSOL (redeem stablecoin)
// ============================================================================

impl InstructionBuilder<HYUSD, JITOSOL> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N9>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::RedeemStablecoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(JITOSOL::MINT),
      fee_vault: pda::fee_vault(JITOSOL::MINT),
      vault_auth: pda::vault_auth(JITOSOL::MINT),
      lst_vault: pda::vault(JITOSOL::MINT),
      lst_header: pda::lst_header(JITOSOL::MINT),
      user_lst_ta: ata!(user, JITOSOL::MINT),
      user_stablecoin_ta: pda::hyusd_ata(user),
      lst_mint: JITOSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::RedeemStablecoin {
      amount_to_redeem: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let redeem_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, JITOSOL::MINT);

    vec![create_ata_ix, redeem_ix]
  }
}

// ============================================================================
// Implementations for HYLOSOL → HYUSD (mint stablecoin)
// ============================================================================

impl InstructionBuilder<HYLOSOL, HYUSD> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::MintStablecoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(HYLOSOL::MINT),
      vault_auth: pda::vault_auth(HYLOSOL::MINT),
      stablecoin_auth: *pda::HYUSD_AUTH,
      fee_vault: pda::fee_vault(HYLOSOL::MINT),
      lst_vault: pda::vault(HYLOSOL::MINT),
      lst_header: pda::lst_header(HYLOSOL::MINT),
      user_lst_ta: ata!(user, HYLOSOL::MINT),
      user_stablecoin_ta: pda::hyusd_ata(user),
      lst_mint: HYLOSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::MintStablecoin {
      amount_lst_to_deposit: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let mint_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, HYUSD::MINT);

    vec![create_ata_ix, mint_ix]
  }
}

// ============================================================================
// Implementations for HYUSD → HYLOSOL (redeem stablecoin)
// ============================================================================

impl InstructionBuilder<HYUSD, HYLOSOL> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N9>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::RedeemStablecoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(HYLOSOL::MINT),
      fee_vault: pda::fee_vault(HYLOSOL::MINT),
      vault_auth: pda::vault_auth(HYLOSOL::MINT),
      lst_vault: pda::vault(HYLOSOL::MINT),
      lst_header: pda::lst_header(HYLOSOL::MINT),
      user_lst_ta: ata!(user, HYLOSOL::MINT),
      user_stablecoin_ta: pda::hyusd_ata(user),
      lst_mint: HYLOSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::RedeemStablecoin {
      amount_to_redeem: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let redeem_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, HYLOSOL::MINT);

    vec![create_ata_ix, redeem_ix]
  }
}

impl InstructionBuilder<JITOSOL, XSOL> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::MintLevercoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(JITOSOL::MINT),
      vault_auth: pda::vault_auth(JITOSOL::MINT),
      levercoin_auth: *pda::XSOL_AUTH,
      fee_vault: pda::fee_vault(JITOSOL::MINT),
      lst_vault: pda::vault(JITOSOL::MINT),
      lst_header: pda::lst_header(JITOSOL::MINT),
      user_lst_ta: ata!(user, JITOSOL::MINT),
      user_levercoin_ta: ata!(user, XSOL::MINT),
      lst_mint: JITOSOL::MINT,
      levercoin_mint: XSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::MintLevercoin {
      amount_lst_to_deposit: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let mint_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, XSOL::MINT);

    vec![create_ata_ix, mint_ix]
  }
}

impl InstructionBuilder<XSOL, JITOSOL> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N9>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::RedeemLevercoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(JITOSOL::MINT),
      fee_vault: pda::fee_vault(JITOSOL::MINT),
      vault_auth: pda::vault_auth(JITOSOL::MINT),
      lst_vault: pda::vault(JITOSOL::MINT),
      lst_header: pda::lst_header(JITOSOL::MINT),
      user_lst_ta: ata!(user, JITOSOL::MINT),
      user_levercoin_ta: ata!(user, XSOL::MINT),
      lst_mint: JITOSOL::MINT,
      levercoin_mint: XSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::RedeemLevercoin {
      amount_to_redeem: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let redeem_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, JITOSOL::MINT);

    vec![create_ata_ix, redeem_ix]
  }
}

impl InstructionBuilder<HYLOSOL, XSOL> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::MintLevercoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(HYLOSOL::MINT),
      vault_auth: pda::vault_auth(HYLOSOL::MINT),
      levercoin_auth: *pda::XSOL_AUTH,
      fee_vault: pda::fee_vault(HYLOSOL::MINT),
      lst_vault: pda::vault(HYLOSOL::MINT),
      lst_header: pda::lst_header(HYLOSOL::MINT),
      user_lst_ta: ata!(user, HYLOSOL::MINT),
      user_levercoin_ta: ata!(user, XSOL::MINT),
      lst_mint: HYLOSOL::MINT,
      levercoin_mint: XSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::MintLevercoin {
      amount_lst_to_deposit: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let mint_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, XSOL::MINT);

    vec![create_ata_ix, mint_ix]
  }
}

impl InstructionBuilder<XSOL, HYLOSOL> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N9>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::RedeemLevercoin {
      user,
      hylo: *pda::HYLO,
      fee_auth: pda::fee_auth(HYLOSOL::MINT),
      fee_vault: pda::fee_vault(HYLOSOL::MINT),
      vault_auth: pda::vault_auth(HYLOSOL::MINT),
      lst_vault: pda::vault(HYLOSOL::MINT),
      lst_header: pda::lst_header(HYLOSOL::MINT),
      user_lst_ta: ata!(user, HYLOSOL::MINT),
      user_levercoin_ta: ata!(user, XSOL::MINT),
      lst_mint: HYLOSOL::MINT,
      levercoin_mint: XSOL::MINT,
      stablecoin_mint: HYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      system_program: system_program::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::RedeemLevercoin {
      amount_to_redeem: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let redeem_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, HYLOSOL::MINT);

    vec![create_ata_ix, redeem_ix]
  }
}

impl InstructionBuilder<HYUSD, XSOL> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::SwapStableToLever {
      user,
      hylo: *pda::HYLO,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      fee_auth: pda::fee_auth(HYUSD::MINT),
      fee_vault: pda::fee_vault(HYUSD::MINT),
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      user_stablecoin_ta: pda::hyusd_ata(user),
      user_levercoin_ta: ata!(user, XSOL::MINT),
      stablecoin_mint: HYUSD::MINT,
      levercoin_mint: XSOL::MINT,
      token_program: anchor_spl::token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::SwapStableToLever {
      amount_stablecoin: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let swap_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, XSOL::MINT);

    vec![create_ata_ix, swap_ix]
  }
}

impl InstructionBuilder<XSOL, HYUSD> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    slippage_bps: u16,
  ) -> Vec<Instruction> {
    let expected_token_out = UFix64::<N6>::new(quote.amount_out);
    let slippage_tolerance = UFix64::<N4>::new(u64::from(slippage_bps));
    let slippage_config =
      SlippageConfig::new(expected_token_out, slippage_tolerance);

    let accounts = ex_accounts::SwapLeverToStable {
      user,
      hylo: *pda::HYLO,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      fee_auth: pda::fee_auth(HYUSD::MINT),
      fee_vault: pda::fee_vault(HYUSD::MINT),
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      user_stablecoin_ta: pda::hyusd_ata(user),
      user_levercoin_ta: ata!(user, XSOL::MINT),
      stablecoin_mint: HYUSD::MINT,
      levercoin_mint: XSOL::MINT,
      token_program: anchor_spl::token::ID,
      event_authority: *pda::EXCHANGE_EVENT_AUTH,
      program: exchange::ID,
    };

    let args = ex_args::SwapLeverToStable {
      amount_levercoin: quote.amount_in,
      slippage_config: Some(slippage_config.into()),
    };

    let swap_ix = Instruction {
      program_id: exchange::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

    let create_ata_ix = create_ata_instruction(user, HYUSD::MINT);

    vec![create_ata_ix, swap_ix]
  }
}

impl InstructionBuilder<HYUSD, SHYUSD> for () {
  fn build(
    &self,
    quote: &QuoteAmounts,
    user: Pubkey,
    _slippage_bps: u16,
  ) -> Vec<Instruction> {
    let accounts = sp_accounts::UserDeposit {
      user,
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: HYUSD::MINT,
      levercoin_mint: XSOL::MINT,
      user_stablecoin_ta: pda::hyusd_ata(user),
      user_lp_token_ta: pda::shyusd_ata(user),
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      lp_token_auth: *pda::SHYUSD_AUTH,
      lp_token_mint: SHYUSD::MINT,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      system_program: system_program::ID,
      token_program: anchor_spl::token::ID,
      associated_token_program: anchor_spl::associated_token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };

    let args = sp_args::UserDeposit {
      amount_stablecoin: quote.amount_in,
    };

    let deposit_ix = Instruction {
      program_id: stability_pool::ID,
      accounts: accounts.to_account_metas(None),
      data: args.data(),
    };

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
