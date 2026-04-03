//! Prints every non-user protocol account for address lookup table
//! creation. Respects the `shadow` feature flag.
//!
//! ```bash
//! cargo run -p hylo-idl --bin lut-accounts
//! cargo run -p hylo-idl --bin lut-accounts --features shadow
//! ```

use anchor_lang::prelude::Pubkey;
use anchor_spl::token;
use hylo_idl::tokens::{
  StakePool, TokenMint, CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC,
  XSOL,
};
use hylo_idl::{exchange, pda, stability_pool};

// Program IDs
// Token mints
// Global PDAs
// Mint authorities
// Event authorities
// Program data
// Stability pool token accounts
// Oracle feeds
// JITOSOL accounts
// HYLOSOL accounts
// hyUSD fee accounts
// USDC accounts
// CBBTC/EXO accounts
// Standard programs
const LUT_ACCOUNTS: &[Pubkey] = &[
  // Program IDs
  exchange::ID,
  stability_pool::ID,
  // Token mints
  HYUSD::MINT,
  XSOL::MINT,
  SHYUSD::MINT,
  XBTC::MINT,
  JITOSOL::MINT,
  HYLOSOL::MINT,
  USDC::MINT,
  CBBTC::MINT,
  // Global PDAs
  pda::HYLO,
  pda::POOL_CONFIG,
  pda::POOL_AUTH,
  pda::LST_REGISTRY_AUTH,
  pda::USDC_PAIR,
  // Mint authorities
  pda::HYUSD_AUTH,
  pda::XSOL_AUTH,
  pda::SHYUSD_AUTH,
  // Event authorities
  pda::EXCHANGE_EVENT_AUTHORITY,
  pda::STABILITY_POOL_EVENT_AUTHORITY,
  // Program data
  pda::EXCHANGE_PROGRAM_DATA,
  pda::STABILITY_POOL_PROGRAM_DATA,
  // Stability pool token accounts
  pda::HYUSD_POOL,
  pda::XSOL_POOL,
  // Oracle feeds
  pda::SOL_USD_PYTH_FEED,
  pda::USDC_USD_PYTH_FEED,
  // JITOSOL accounts
  pda::fee_auth(JITOSOL::MINT),
  pda::lst_vault_auth(JITOSOL::MINT),
  pda::fee_vault(JITOSOL::MINT),
  pda::lst_vault(JITOSOL::MINT),
  pda::lst_header(JITOSOL::MINT),
  JITOSOL::POOL_STATE,
  // HYLOSOL accounts
  pda::fee_auth(HYLOSOL::MINT),
  pda::lst_vault_auth(HYLOSOL::MINT),
  pda::fee_vault(HYLOSOL::MINT),
  pda::lst_vault(HYLOSOL::MINT),
  pda::lst_header(HYLOSOL::MINT),
  HYLOSOL::POOL_STATE,
  // hyUSD fee accounts
  pda::fee_auth(HYUSD::MINT),
  pda::fee_vault(HYUSD::MINT),
  // USDC accounts
  pda::usdc_vault_auth(USDC::MINT),
  pda::fee_auth(USDC::MINT),
  pda::ata(pda::usdc_vault_auth(USDC::MINT), USDC::MINT),
  pda::ata(pda::fee_auth(USDC::MINT), USDC::MINT),
  // CBBTC/EXO accounts
  pda::exo_pair(CBBTC::MINT),
  pda::exo_vault_auth(CBBTC::MINT),
  pda::mint_auth(XBTC::MINT),
  pda::fee_auth(CBBTC::MINT),
  pda::ata(pda::exo_vault_auth(CBBTC::MINT), CBBTC::MINT),
  pda::ata(pda::fee_auth(CBBTC::MINT), CBBTC::MINT),
  // Standard programs
  token::ID,
  mpl_token_metadata::ID,
];

#[allow(clippy::needless_for_each)]
fn main() {
  LUT_ACCOUNTS.iter().for_each(|key| println!("{key}"));
}
