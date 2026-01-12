use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_lang::prelude::{AccountDeserialize, Pubkey};
use anchor_lang::solana_program::sysvar::clock::{self, Clock};
use anyhow::{anyhow, Result};
use fix::prelude::*;
use fix::typenum::{IsLess, NInt, NonZero, Unsigned, U20};
use hylo_clients::token_operation::OperationOutput;
use hylo_jupiter_amm_interface::{
  AccountMap, AmmContext, ClockRef, Quote, SwapMode, SwapParams,
};
use rust_decimal::Decimal;

/// Computes fee percentage as `Decimal`.
///
/// # Errors
/// * Arithmetic overflow
/// * u64 to i64 conversion
pub fn fee_pct_decimal<Exp>(
  fees_extracted: UFix64<NInt<Exp>>,
  total_in: UFix64<NInt<Exp>>,
) -> Result<Decimal>
where
  Exp: Unsigned + NonZero + IsLess<U20>,
{
  let pct_fix = fees_extracted
    .mul_div_floor(UFix64::one(), total_in)
    .ok_or(anyhow!("Arithmetic error in fee_pct calculation"))?;
  Ok(Decimal::new(pct_fix.bits.try_into()?, Exp::to_u32()))
}

/// Converts [`OperationOutput`] to Jupiter [`Quote`].
#[must_use]
pub fn operation_to_quote(
  OperationOutput {
    in_amount,
    out_amount,
    fee_amount,
    fee_mint,
    fee_base,
  }: OperationOutput,
) -> Quote {
  let fee_pct = if fee_base == 0 {
    Decimal::ZERO
  } else {
    Decimal::from(fee_amount) / Decimal::from(fee_base)
  };
  Quote {
    in_amount,
    out_amount,
    fee_amount,
    fee_mint,
    fee_pct,
  }
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
  let account = account_map
    .get(key)
    .ok_or(anyhow!("Account not found {key}"))?;
  let mut bytes = account.data.as_slice();
  let out = A::try_deserialize(&mut bytes)?;
  Ok(out)
}

/// Calls RPC to load given accounts into a map.
///
/// # Errors
/// * RPC fails
/// * One of the accounts is missing
pub async fn load_account_map(
  client: &RpcClient,
  pubkeys: &[Pubkey],
) -> Result<AccountMap> {
  let accounts = client.get_multiple_accounts(pubkeys).await?;
  pubkeys
    .iter()
    .zip(accounts)
    .map(|(pubkey, account)| {
      account
        .ok_or_else(|| anyhow!("Account not found: {pubkey}"))
        .map(|acc| (*pubkey, acc))
    })
    .collect::<Result<AccountMap>>()
}

/// Loads Solana clock information from RPC.
///
/// # Errors
/// * RPC fails
/// * Deserialization fails
pub async fn load_amm_context(client: &RpcClient) -> Result<AmmContext> {
  let clock_account = client.get_account(&clock::ID).await?;
  let clock: Clock = bincode::deserialize(&clock_account.data)?;
  let clock_ref = ClockRef::from(clock);
  Ok(AmmContext { clock_ref })
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
