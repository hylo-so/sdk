//! `QuoteStrategy` implementations for exchange pairs using `TokenOperation`.

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::*;
use hylo_clients::instructions::ExchangeInstructionBuilder as ExchangeIB;
use hylo_clients::instructions::RouterInstructionBuilder as RouterIB;
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::transaction::{
  LstSwapArgs, MintArgs, RedeemArgs, RouterArgs, SwapArgs,
};
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{CBBTC, HYUSD, USDC, XBTC, XSOL};

use crate::protocol_state::{ProtocolState, StateProvider};
use crate::protocol_state_strategy::ProtocolStateStrategy;
use crate::token_operation::{TokenOperation, TokenOperationExt};
use crate::{
  ComputeUnitStrategy, ExecutableQuote, Local, QuoteStrategy,
  DEFAULT_CUS_WITH_BUFFER, LST,
};

type MintQuote = ExecutableQuote<N9, N6, N9>;
type RedeemQuote = ExecutableQuote<N6, N9, N9>;
type SwapQuote = ExecutableQuote<N6, N6, N6>;
type LstSwapQuote = ExecutableQuote<N9, N9, N9>;
type ExoMintQuote = ExecutableQuote<N8, N6, N9>;
type ExoRedeemQuote = ExecutableQuote<N6, N8, N9>;

// LST -> HYUSD (mint stablecoin)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<L, HYUSD, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<L, HYUSD, FeeExp = N9>,
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<MintQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.compute_output(UFix64::new(amount_in))?;
    let args = MintArgs {
      amount: UFix64::<N9>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<L, HYUSD>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L, HYUSD>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// HYUSD -> LST (redeem stablecoin)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<HYUSD, L, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<HYUSD, L, FeeExp = N9>,
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<RedeemQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.compute_output(UFix64::new(amount_in))?;
    let args = RedeemArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<HYUSD, L>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<HYUSD, L>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// LST -> XSOL (mint levercoin)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<L, XSOL, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<L, XSOL, FeeExp = N9>,
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<MintQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.compute_output(UFix64::new(amount_in))?;
    let args = MintArgs {
      amount: UFix64::<N9>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<L, XSOL>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L, XSOL>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// XSOL -> LST (redeem levercoin)
#[async_trait]
impl<L: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<XSOL, L, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<XSOL, L, FeeExp = N9>,
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<RedeemQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.compute_output(UFix64::new(amount_in))?;
    let args = RedeemArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<XSOL, L>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<XSOL, L>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// HYUSD -> XSOL (swap stable to lever)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<HYUSD, XSOL, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<SwapQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<HYUSD, XSOL>(UFix64::new(amount_in))?;
    let args = SwapArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<HYUSD, XSOL>(args)?;
    let address_lookup_tables =
      ExchangeIB::lookup_tables::<HYUSD, XSOL>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// XSOL -> HYUSD (swap lever to stable)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<XSOL, HYUSD, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<SwapQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<XSOL, HYUSD>(UFix64::new(amount_in))?;
    let args = SwapArgs {
      amount: UFix64::<N6>::new(amount_in),
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<XSOL, HYUSD>(args)?;
    let address_lookup_tables =
      ExchangeIB::lookup_tables::<XSOL, HYUSD>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// LST -> LST swap
#[async_trait]
impl<L1: LST + Local, L2: LST + Local, S: StateProvider<C>, C: SolanaClock>
  QuoteStrategy<L1, L2, C> for ProtocolStateStrategy<S>
where
  ProtocolState<C>: TokenOperation<L1, L2, FeeExp = N9>,
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<LstSwapQuote> {
    let state = self.state_provider.fetch_state().await?;
    let amount = UFix64::<N9>::new(amount_in);
    let op = state.compute_output(amount)?;
    let args = LstSwapArgs {
      amount_lst_a: amount,
      lst_a_mint: L1::MINT,
      lst_b_mint: L2::MINT,
      user,
      slippage_config: Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    };
    let instructions = ExchangeIB::build_instructions::<L1, L2>(args)?;
    let address_lookup_tables = ExchangeIB::lookup_tables::<L1, L2>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// ============================================================================
// Exo / USDC routes (router-based)
// ============================================================================

/// Builds `RouterArgs` for a router-based quote.
fn router_args(
  amount_in: u64,
  user: Pubkey,
  slippage_config: Option<SlippageConfig>,
) -> RouterArgs {
  RouterArgs {
    amount: amount_in,
    user,
    slippage_config,
  }
}

// USDC -> HYUSD (mint stablecoin from USDC)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<USDC, HYUSD, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<SwapQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<USDC, HYUSD>(UFix64::new(amount_in))?;
    let args = router_args(
      amount_in,
      user,
      Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    );
    let instructions = RouterIB::build_instructions::<USDC, HYUSD>(args)?;
    let address_lookup_tables = RouterIB::lookup_tables::<USDC, HYUSD>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// HYUSD -> USDC (redeem stablecoin to USDC)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<HYUSD, USDC, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<SwapQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<HYUSD, USDC>(UFix64::new(amount_in))?;
    let args = router_args(
      amount_in,
      user,
      Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    );
    let instructions = RouterIB::build_instructions::<HYUSD, USDC>(args)?;
    let address_lookup_tables = RouterIB::lookup_tables::<HYUSD, USDC>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// CBBTC -> HYUSD (mint stablecoin from exo)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<CBBTC, HYUSD, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<ExoMintQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<CBBTC, HYUSD>(UFix64::new(amount_in))?;
    let args = router_args(
      amount_in,
      user,
      Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    );
    let instructions = RouterIB::build_instructions::<CBBTC, HYUSD>(args)?;
    let address_lookup_tables =
      RouterIB::lookup_tables::<CBBTC, HYUSD>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// HYUSD -> CBBTC (redeem stablecoin for exo)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<HYUSD, CBBTC, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<ExoRedeemQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<HYUSD, CBBTC>(UFix64::new(amount_in))?;
    let args = router_args(
      amount_in,
      user,
      Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    );
    let instructions = RouterIB::build_instructions::<HYUSD, CBBTC>(args)?;
    let address_lookup_tables =
      RouterIB::lookup_tables::<HYUSD, CBBTC>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// CBBTC -> XBTC (mint levercoin from exo)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<CBBTC, XBTC, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<ExoMintQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<CBBTC, XBTC>(UFix64::new(amount_in))?;
    let args = router_args(
      amount_in,
      user,
      Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    );
    let instructions = RouterIB::build_instructions::<CBBTC, XBTC>(args)?;
    let address_lookup_tables = RouterIB::lookup_tables::<CBBTC, XBTC>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// XBTC -> CBBTC (redeem levercoin for exo)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<XBTC, CBBTC, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N9;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<ExoRedeemQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<XBTC, CBBTC>(UFix64::new(amount_in))?;
    let args = router_args(
      amount_in,
      user,
      Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    );
    let instructions = RouterIB::build_instructions::<XBTC, CBBTC>(args)?;
    let address_lookup_tables = RouterIB::lookup_tables::<XBTC, CBBTC>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// HYUSD -> XBTC (swap stable to exo lever)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<HYUSD, XBTC, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<SwapQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<HYUSD, XBTC>(UFix64::new(amount_in))?;
    let args = router_args(
      amount_in,
      user,
      Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    );
    let instructions = RouterIB::build_instructions::<HYUSD, XBTC>(args)?;
    let address_lookup_tables = RouterIB::lookup_tables::<HYUSD, XBTC>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}

// XBTC -> HYUSD (swap exo lever to stable)
#[async_trait]
impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<XBTC, HYUSD, C>
  for ProtocolStateStrategy<S>
{
  type FeeExp = N6;

  async fn get_quote(
    &self,
    amount_in: u64,
    user: Pubkey,
    slippage_tolerance: u64,
  ) -> Result<SwapQuote> {
    let state = self.state_provider.fetch_state().await?;
    let op = state.output::<XBTC, HYUSD>(UFix64::new(amount_in))?;
    let args = router_args(
      amount_in,
      user,
      Some(SlippageConfig::new(
        op.out_amount,
        UFix64::<N4>::new(slippage_tolerance),
      )),
    );
    let instructions = RouterIB::build_instructions::<XBTC, HYUSD>(args)?;
    let address_lookup_tables = RouterIB::lookup_tables::<XBTC, HYUSD>().into();
    Ok(ExecutableQuote {
      amount_in: op.in_amount,
      amount_out: op.out_amount,
      compute_units: DEFAULT_CUS_WITH_BUFFER,
      compute_unit_strategy: ComputeUnitStrategy::Estimated,
      fee_amount: op.fee_amount,
      fee_mint: op.fee_mint,
      instructions,
      address_lookup_tables,
    })
  }
}
