use anchor_client::solana_sdk::account::Account;
use anchor_lang::prelude::{AccountDeserialize, Pubkey};
use anyhow::{anyhow, Context, Result};
use fix::prelude::UFix64;
use hylo_jupiter_amm_interface::{AccountMap, SwapMode, SwapParams};
use rust_decimal::Decimal;

/// Fee percentage of `fee_base` taken as `fees_extracted`, both in atoms of
/// the same mint.
///
/// # Errors
/// * Arithmetic
pub fn fee_pct(fees_extracted: u64, fee_base: u64) -> Result<Decimal> {
  if fee_base == 0 {
    Ok(Decimal::ZERO)
  } else {
    Decimal::from(fees_extracted)
      .checked_div(Decimal::from(fee_base))
      .context("Arithmetic error in `fee_pct`")
  }
}

/// Computes fee percentage as `Decimal`.
///
/// # Errors
/// * Arithmetic
pub fn fee_pct_decimal<Exp>(
  fees_extracted: UFix64<Exp>,
  fee_base: UFix64<Exp>,
) -> Result<Decimal> {
  fee_pct(fees_extracted.bits, fee_base.bits)
}

/// Finds a raw account in Jupiter's `AccountMap`.
///
/// # Errors
/// * Account not found in map
pub fn keyed_account<'a>(
  account_map: &'a AccountMap,
  key: &Pubkey,
) -> Result<&'a Account> {
  account_map
    .get(key)
    .ok_or_else(|| anyhow!("Account not found {key}"))
}

/// Finds and deserializes an account in Jupiter's `AccountMap`.
///
/// # Errors
/// * Account not found in map
/// * Deserialization to `A` fails
pub fn account_map_get<A: AccountDeserialize>(
  account_map: &AccountMap,
  key: &Pubkey,
) -> Result<A> {
  let mut bytes = keyed_account(account_map, key)?.data.as_slice();
  let out = A::try_deserialize(&mut bytes)?;
  Ok(out)
}

/// Validates Jupiter swap parameters for Hylo compatibility.
///
/// # Errors
/// * `ExactOut` mode
/// * Dynamic accounts
pub fn validate_swap_params<'a>(
  params: &'a SwapParams<'a, 'a>,
) -> Result<&'a SwapParams<'a, 'a>> {
  if params.swap_mode == SwapMode::ExactOut {
    Err(anyhow!("ExactOut not supported"))
  } else if params.missing_dynamic_accounts_as_default {
    Err(anyhow!("Dynamic accounts replacement not supported"))
  } else {
    Ok(params)
  }
}
