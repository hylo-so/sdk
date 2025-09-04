use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_lang::prelude::{AccountDeserialize, Pubkey};
use anchor_lang::solana_program::sysvar::clock::{self, Clock};
use anyhow::{anyhow, Result};
use fix::prelude::*;
use fix::typenum::{IsLess, NInt, NonZero, Unsigned, U20};
use jupiter_amm_interface::{AccountMap, AmmContext, ClockRef};
use rust_decimal::Decimal;

/// Computes fee percentage in Jupiter's favored `Decimal` type.
///
/// # Errors
/// * Arithmetic error for percentage
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
    .ok_or(anyhow!("Account not found {}", key))?;
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
