//! `QuoteStrategy` simulation impls routed through [`RouterClient`].

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N4, N6, N8, N9};
use fix::typenum::Integer;
use hylo_clients::router_client::{
  InstructionBuilderExt, RouterArgs, RouterClient,
};
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{
  CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC, XSOL,
};

use crate::simulated_operation::SimulatedOperationExt;
use crate::simulation_strategy::SimulationStrategy;
use crate::{ExecutableQuote, QuoteStrategy};

fn sim_args(amount_in: u64, user: Pubkey) -> RouterArgs {
  RouterArgs {
    amount: amount_in,
    user,
    slippage_config: None,
  }
}

fn quote_args<E: Integer>(
  amount_in: u64,
  user: Pubkey,
  out_amount: UFix64<E>,
  slippage_tolerance: u64,
) -> RouterArgs {
  RouterArgs {
    amount: amount_in,
    user,
    slippage_config: Some(SlippageConfig::new(
      out_amount,
      UFix64::<N4>::new(slippage_tolerance),
    )),
  }
}

macro_rules! simulation_quote {
  ($in:ty, $out:ty, $fee_exp:ty, $quote_ty:ty) => {
    #[async_trait]
    impl<C: SolanaClock> QuoteStrategy<$in, $out, C> for SimulationStrategy {
      type FeeExp = $fee_exp;

      async fn get_quote(
        &self,
        amount_in: u64,
        user: Pubkey,
        slippage_tolerance: u64,
      ) -> Result<$quote_ty> {
        let (output, cu_info) = self
          .router_client
          .simulate_output::<$in, $out>(user, sim_args(amount_in, user))
          .await?;
        let args =
          quote_args(amount_in, user, output.out_amount, slippage_tolerance);
        let instructions = RouterClient::build_instructions::<$in, $out>(args)?;
        let address_lookup_tables =
          RouterClient::lookup_tables::<$in, $out>().into();
        Ok(ExecutableQuote {
          amount_in: output.in_amount,
          amount_out: output.out_amount,
          compute_units: cu_info.compute_units,
          compute_unit_strategy: cu_info.strategy,
          fee_amount: output.fee_amount,
          fee_mint: output.fee_mint,
          instructions,
          address_lookup_tables,
        })
      }
    }
  };
}

// `mint_stablecoin_lst`
simulation_quote!(JITOSOL, HYUSD, N9, ExecutableQuote<N9, N6, N9>);
simulation_quote!(HYLOSOL, HYUSD, N9, ExecutableQuote<N9, N6, N9>);

// `redeem_stablecoin_lst`
simulation_quote!(HYUSD, JITOSOL, N9, ExecutableQuote<N6, N9, N9>);
simulation_quote!(HYUSD, HYLOSOL, N9, ExecutableQuote<N6, N9, N9>);

// `mint_levercoin_lst`
simulation_quote!(JITOSOL, XSOL, N9, ExecutableQuote<N9, N6, N9>);
simulation_quote!(HYLOSOL, XSOL, N9, ExecutableQuote<N9, N6, N9>);

// `redeem_levercoin_lst`
simulation_quote!(XSOL, JITOSOL, N9, ExecutableQuote<N6, N9, N9>);
simulation_quote!(XSOL, HYLOSOL, N9, ExecutableQuote<N6, N9, N9>);

// `convert_stable_to_lever_lst`
simulation_quote!(HYUSD, XSOL, N6, ExecutableQuote<N6, N6, N6>);

// `convert_lever_to_stable_lst`
simulation_quote!(XSOL, HYUSD, N6, ExecutableQuote<N6, N6, N6>);

// `swap_lst_to_lst`
simulation_quote!(JITOSOL, HYLOSOL, N9, ExecutableQuote<N9, N9, N9>);
simulation_quote!(HYLOSOL, JITOSOL, N9, ExecutableQuote<N9, N9, N9>);

// `mint_stablecoin_usdc`
simulation_quote!(USDC, HYUSD, N9, ExecutableQuote<N6, N6, N9>);

// `redeem_stablecoin_usdc`
simulation_quote!(HYUSD, USDC, N6, ExecutableQuote<N6, N6, N6>);

// `mint_stablecoin_exo`
simulation_quote!(CBBTC, HYUSD, N9, ExecutableQuote<N8, N6, N9>);

// `redeem_stablecoin_exo`
simulation_quote!(HYUSD, CBBTC, N9, ExecutableQuote<N6, N8, N9>);

// `mint_levercoin_exo`
simulation_quote!(CBBTC, XBTC, N9, ExecutableQuote<N8, N6, N9>);

// `redeem_levercoin_exo`
simulation_quote!(XBTC, CBBTC, N9, ExecutableQuote<N6, N8, N9>);

// `convert_stable_to_lever_exo`
simulation_quote!(HYUSD, XBTC, N6, ExecutableQuote<N6, N6, N6>);

// `convert_lever_to_stable_exo`
simulation_quote!(XBTC, HYUSD, N6, ExecutableQuote<N6, N6, N6>);

// `swap_lst_to_usdc`
simulation_quote!(JITOSOL, USDC, N9, ExecutableQuote<N9, N6, N9>);
simulation_quote!(HYLOSOL, USDC, N9, ExecutableQuote<N9, N6, N9>);

// `swap_usdc_to_lst`
simulation_quote!(USDC, JITOSOL, N6, ExecutableQuote<N6, N9, N6>);
simulation_quote!(USDC, HYLOSOL, N6, ExecutableQuote<N6, N9, N6>);

// `swap_exo_to_usdc`
simulation_quote!(CBBTC, USDC, N8, ExecutableQuote<N8, N6, N8>);

// `swap_usdc_to_exo`
simulation_quote!(USDC, CBBTC, N6, ExecutableQuote<N6, N8, N6>);

// `user_deposit`
simulation_quote!(HYUSD, SHYUSD, N6, ExecutableQuote<N6, N6, N6>);

// `user_withdraw`
simulation_quote!(SHYUSD, HYUSD, N6, ExecutableQuote<N6, N6, N6>);
