//! `QuoteStrategy` state-based impls routed through [`RouterClient`].

use anchor_lang::prelude::Pubkey;
use anyhow::Result;
use async_trait::async_trait;
use fix::prelude::*;
use hylo_clients::router_client::{
  InstructionBuilderExt, RouterArgs, RouterClient,
};
use hylo_core::slippage_config::SlippageConfig;
use hylo_core::solana_clock::SolanaClock;
use hylo_idl::tokens::{
  CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC, XSOL,
};

use crate::protocol_state::StateProvider;
use crate::protocol_state_strategy::ProtocolStateStrategy;
use crate::token_operation::TokenOperationExt;
use crate::{
  ComputeUnitStrategy, ExecutableQuote, QuoteStrategy, DEFAULT_CUS_WITH_BUFFER,
};

macro_rules! state_quote {
  ($in:ty, $out:ty, $fee_exp:ty, $quote_ty:ty) => {
    #[async_trait]
    impl<S: StateProvider<C>, C: SolanaClock> QuoteStrategy<$in, $out, C>
      for ProtocolStateStrategy<S>
    {
      type FeeExp = $fee_exp;

      async fn get_quote(
        &self,
        amount_in: u64,
        user: Pubkey,
        slippage_tolerance: u64,
      ) -> Result<$quote_ty> {
        let state = self.state_provider.fetch_state().await?;
        let op = state.output::<$in, $out>(UFix64::new(amount_in))?;
        let args = RouterArgs {
          amount: amount_in,
          user,
          slippage_config: Some(SlippageConfig::new(
            op.out_amount,
            UFix64::<N4>::new(slippage_tolerance),
          )),
        };
        let instructions = RouterClient::build_instructions::<$in, $out>(args)?;
        let address_lookup_tables =
          RouterClient::lookup_tables::<$in, $out>().into();
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
  };
}

// `mint_stablecoin_lst`
state_quote!(JITOSOL, HYUSD, N9, ExecutableQuote<N9, N6, N9>);
state_quote!(HYLOSOL, HYUSD, N9, ExecutableQuote<N9, N6, N9>);

// `redeem_stablecoin_lst`
state_quote!(HYUSD, JITOSOL, N9, ExecutableQuote<N6, N9, N9>);
state_quote!(HYUSD, HYLOSOL, N9, ExecutableQuote<N6, N9, N9>);

// `mint_levercoin_lst`
state_quote!(JITOSOL, XSOL, N9, ExecutableQuote<N9, N6, N9>);
state_quote!(HYLOSOL, XSOL, N9, ExecutableQuote<N9, N6, N9>);

// `redeem_levercoin_lst`
state_quote!(XSOL, JITOSOL, N9, ExecutableQuote<N6, N9, N9>);
state_quote!(XSOL, HYLOSOL, N9, ExecutableQuote<N6, N9, N9>);

// `convert_stable_to_lever_lst`
state_quote!(HYUSD, XSOL, N6, ExecutableQuote<N6, N6, N6>);

// `convert_lever_to_stable_lst`
state_quote!(XSOL, HYUSD, N6, ExecutableQuote<N6, N6, N6>);

// `swap_lst_to_lst`
state_quote!(JITOSOL, HYLOSOL, N9, ExecutableQuote<N9, N9, N9>);
state_quote!(HYLOSOL, JITOSOL, N9, ExecutableQuote<N9, N9, N9>);

// `mint_stablecoin_usdc`
state_quote!(USDC, HYUSD, N9, ExecutableQuote<N6, N6, N9>);

// `redeem_stablecoin_usdc`
state_quote!(HYUSD, USDC, N6, ExecutableQuote<N6, N6, N6>);

// `mint_stablecoin_exo`
state_quote!(CBBTC, HYUSD, N9, ExecutableQuote<N8, N6, N9>);

// `redeem_stablecoin_exo`
state_quote!(HYUSD, CBBTC, N9, ExecutableQuote<N6, N8, N9>);

// `mint_levercoin_exo`
state_quote!(CBBTC, XBTC, N9, ExecutableQuote<N8, N6, N9>);

// `redeem_levercoin_exo`
state_quote!(XBTC, CBBTC, N9, ExecutableQuote<N6, N8, N9>);

// `convert_stable_to_lever_exo`
state_quote!(HYUSD, XBTC, N6, ExecutableQuote<N6, N6, N6>);

// `convert_lever_to_stable_exo`
state_quote!(XBTC, HYUSD, N6, ExecutableQuote<N6, N6, N6>);

// `swap_lst_to_usdc`
state_quote!(JITOSOL, USDC, N9, ExecutableQuote<N9, N6, N9>);
state_quote!(HYLOSOL, USDC, N9, ExecutableQuote<N9, N6, N9>);

// `swap_usdc_to_lst`
state_quote!(USDC, JITOSOL, N6, ExecutableQuote<N6, N9, N6>);
state_quote!(USDC, HYLOSOL, N6, ExecutableQuote<N6, N9, N6>);

// `swap_exo_to_usdc`
state_quote!(CBBTC, USDC, N8, ExecutableQuote<N8, N6, N8>);

// `swap_usdc_to_exo`
state_quote!(USDC, CBBTC, N6, ExecutableQuote<N6, N8, N6>);

// `user_deposit`
state_quote!(HYUSD, SHYUSD, N6, ExecutableQuote<N6, N6, N6>);

// `user_withdraw`
state_quote!(SHYUSD, HYUSD, N6, ExecutableQuote<N6, N6, N6>);
