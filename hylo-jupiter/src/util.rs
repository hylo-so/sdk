use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_lang::prelude::{AccountDeserialize, Pubkey};
use anchor_lang::solana_program::sysvar::clock::{self, Clock};
use anyhow::{anyhow, Context, Result};
use fix::num_traits::FromPrimitive;
use fix::prelude::UFix64;
use fix::typenum::Integer;
use hylo_clients::protocol_state::ProtocolState;
use hylo_clients::token_operation::{OperationOutput, TokenOperation};
use hylo_core::idl::tokens::TokenMint;
use hylo_jupiter_amm_interface::{
  AccountMap, AmmContext, ClockRef, Quote, SwapMode, SwapParams,
};
use rust_decimal::Decimal;

/// Computes fee percentage as `Decimal`.
///
/// # Errors
/// * Conversions
/// * Arithmetic
pub fn fee_pct_decimal<Exp>(
  fees_extracted: UFix64<Exp>,
  fee_base: UFix64<Exp>,
) -> Result<Decimal> {
  if fee_base == UFix64::new(0) {
    Ok(Decimal::ZERO)
  } else {
    Decimal::from_u64(fees_extracted.bits)
      .zip(Decimal::from_u64(fee_base.bits))
      .and_then(|(num, denom)| num.checked_div(denom))
      .context("Arithmetic error in `fee_pct_decimal`")
  }
}

/// Converts [`OperationOutput`] to Jupiter [`Quote`].
///
/// # Errors
/// * Fee decimal conversion
pub fn operation_to_quote<InExp, OutExp, FeeExp>(
  op: OperationOutput<InExp, OutExp, FeeExp>,
) -> Result<Quote>
where
  InExp: Integer,
  OutExp: Integer,
  FeeExp: Integer,
{
  let fee_pct = fee_pct_decimal(op.fee_amount, op.fee_base)?;
  Ok(Quote {
    in_amount: op.in_amount.bits,
    out_amount: op.out_amount.bits,
    fee_amount: op.fee_amount.bits,
    fee_mint: op.fee_mint,
    fee_pct,
  })
}

/// Generic Jupiter quote for any `IN -> OUT` pair.
///
/// # Errors
/// * Quote math
/// * Fee decimal conversion
pub fn quote<IN, OUT>(
  state: &ProtocolState<ClockRef>,
  amount: u64,
) -> Result<Quote>
where
  IN: TokenMint,
  OUT: TokenMint,
  ProtocolState<ClockRef>: TokenOperation<IN, OUT>,
  <ProtocolState<ClockRef> as TokenOperation<IN, OUT>>::FeeExp: Integer,
{
  let op = <ProtocolState<_> as TokenOperation<IN, OUT>>::compute_quote(
    state,
    UFix64::new(amount),
  )?;
  operation_to_quote(op)
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
