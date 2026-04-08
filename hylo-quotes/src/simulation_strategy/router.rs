//! `QuoteStrategy` simulation impls routed through [`RouterClient`].

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::{UFix64, N4, N6, N8, N9};
use fix::typenum::Integer;
use hylo_clients::router_client::RouterClient;
use hylo_clients::syntax_helpers::InstructionBuilderExt;
use hylo_clients::transaction::RouterArgs;
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{
  CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC, XSOL,
};

use crate::simulated_operation::SimulatedOperationExt;
use crate::simulation_strategy::SimulationStrategy;
use crate::{ExecutableQuote, QuoteStrategy};

type MintQuote = ExecutableQuote<N9, N6, N9>;
type RedeemQuote = ExecutableQuote<N6, N9, N9>;
type SwapQuote = ExecutableQuote<N6, N6, N6>;
type LstSwapQuote = ExecutableQuote<N9, N9, N9>;
type ExoMintQuote = ExecutableQuote<N8, N6, N9>;
type ExoRedeemQuote = ExecutableQuote<N6, N8, N9>;
type UsdcMintQuote = ExecutableQuote<N6, N6, N9>;
type SwapLstToUsdcQuote = ExecutableQuote<N9, N6, N9>;
type SwapUsdcToLstQuote = ExecutableQuote<N6, N9, N6>;
type SwapExoToUsdcQuote = ExecutableQuote<N8, N6, N8>;
type SwapUsdcToExoQuote = ExecutableQuote<N6, N8, N6>;

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
simulation_quote!(JITOSOL, HYUSD, N9, MintQuote);
simulation_quote!(HYLOSOL, HYUSD, N9, MintQuote);

// `redeem_stablecoin_lst`
simulation_quote!(HYUSD, JITOSOL, N9, RedeemQuote);
simulation_quote!(HYUSD, HYLOSOL, N9, RedeemQuote);

// `mint_levercoin_lst`
simulation_quote!(JITOSOL, XSOL, N9, MintQuote);
simulation_quote!(HYLOSOL, XSOL, N9, MintQuote);

// `redeem_levercoin_lst`
simulation_quote!(XSOL, JITOSOL, N9, RedeemQuote);
simulation_quote!(XSOL, HYLOSOL, N9, RedeemQuote);

// `convert_stable_to_lever_lst`
simulation_quote!(HYUSD, XSOL, N6, SwapQuote);

// `convert_lever_to_stable_lst`
simulation_quote!(XSOL, HYUSD, N6, SwapQuote);

// `swap_lst_to_lst`
simulation_quote!(JITOSOL, HYLOSOL, N9, LstSwapQuote);
simulation_quote!(HYLOSOL, JITOSOL, N9, LstSwapQuote);

// `mint_stablecoin_usdc`
simulation_quote!(USDC, HYUSD, N9, UsdcMintQuote);

// `redeem_stablecoin_usdc`
simulation_quote!(HYUSD, USDC, N6, SwapQuote);

// `mint_stablecoin_exo`
simulation_quote!(CBBTC, HYUSD, N9, ExoMintQuote);

// `redeem_stablecoin_exo`
simulation_quote!(HYUSD, CBBTC, N9, ExoRedeemQuote);

// `mint_levercoin_exo`
simulation_quote!(CBBTC, XBTC, N9, ExoMintQuote);

// `redeem_levercoin_exo`
simulation_quote!(XBTC, CBBTC, N9, ExoRedeemQuote);

// `convert_stable_to_lever_exo`
simulation_quote!(HYUSD, XBTC, N6, SwapQuote);

// `convert_lever_to_stable_exo`
simulation_quote!(XBTC, HYUSD, N6, SwapQuote);

// `swap_lst_to_usdc`
simulation_quote!(JITOSOL, USDC, N9, SwapLstToUsdcQuote);
simulation_quote!(HYLOSOL, USDC, N9, SwapLstToUsdcQuote);

// `swap_usdc_to_lst`
simulation_quote!(USDC, JITOSOL, N6, SwapUsdcToLstQuote);
simulation_quote!(USDC, HYLOSOL, N6, SwapUsdcToLstQuote);

// `swap_exo_to_usdc`
simulation_quote!(CBBTC, USDC, N8, SwapExoToUsdcQuote);

// `swap_usdc_to_exo`
simulation_quote!(USDC, CBBTC, N6, SwapUsdcToExoQuote);

// `user_deposit`
simulation_quote!(HYUSD, SHYUSD, N6, SwapQuote);

// `user_withdraw`
simulation_quote!(SHYUSD, HYUSD, N6, SwapQuote);
