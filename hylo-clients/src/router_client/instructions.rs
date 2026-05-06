//! [`InstructionBuilder`] impls for [`RouterClient`](super::RouterClient).

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_lang::ToAccountMetas;
use anyhow::Result;
use hylo_core::slippage_config::SlippageConfig;
use hylo_idl::earn_pool::account_builders as ep_account_builders;
use hylo_idl::exchange::account_builders;
use hylo_idl::pda;
use hylo_idl::router::client::args as router_args;
use hylo_idl::router::instruction_builders::route;
use hylo_idl::tokens::{
  StakePool, TokenMint, CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC,
  XSOL,
};

use super::{InstructionBuilder, RouterArgs, RouterClient};
use crate::util::{
  user_ata_instruction, HYLO_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE,
};

const BASE_LOOKUP_TABLES: &[Pubkey] = &[HYLO_LOOKUP_TABLE];
const LST_LOOKUP_TABLES: &[Pubkey] =
  &[HYLO_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE];

fn route_instruction<A: ToAccountMetas>(
  token_a: Pubkey,
  token_b: Pubkey,
  amount: u64,
  slippage_config: Option<SlippageConfig>,
  inner_accounts: &A,
) -> Instruction {
  let args = router_args::Route {
    token_a,
    token_b,
    amount,
    slippage_config: slippage_config.map(Into::into),
  };
  route(&args, inner_accounts)
}

macro_rules! router_instruction {
  ($in:ty, $out:ty, $luts:expr, $ata:expr, |$user:ident| $accts:expr $(,)?) => {
    impl InstructionBuilder<$in, $out> for RouterClient {
      type Inputs = RouterArgs;
      const REQUIRED_LOOKUP_TABLES: &'static [Pubkey] = $luts;

      fn build(
        RouterArgs {
          amount,
          $user,
          slippage_config,
        }: RouterArgs,
      ) -> Result<Vec<Instruction>> {
        let ata = user_ata_instruction(&$user, &$ata);
        let accounts = $accts;
        let ix = route_instruction(
          <$in>::MINT,
          <$out>::MINT,
          amount,
          slippage_config,
          &accounts,
        );
        Ok(vec![ata, ix])
      }
    }
  };
}

// `mint_stablecoin_lst`
router_instruction!(JITOSOL, HYUSD, LST_LOOKUP_TABLES, HYUSD::MINT, |user| {
  account_builders::mint_stablecoin_lst(user, JITOSOL::MINT)
});
router_instruction!(HYLOSOL, HYUSD, LST_LOOKUP_TABLES, HYUSD::MINT, |user| {
  account_builders::mint_stablecoin_lst(user, HYLOSOL::MINT)
});

// `redeem_stablecoin_lst`
router_instruction!(HYUSD, JITOSOL, LST_LOOKUP_TABLES, JITOSOL::MINT, |user| {
  account_builders::redeem_stablecoin_lst(user, JITOSOL::MINT)
});
router_instruction!(HYUSD, HYLOSOL, LST_LOOKUP_TABLES, HYLOSOL::MINT, |user| {
  account_builders::redeem_stablecoin_lst(user, HYLOSOL::MINT)
});

// `mint_levercoin_lst`
router_instruction!(JITOSOL, XSOL, LST_LOOKUP_TABLES, XSOL::MINT, |user| {
  account_builders::mint_levercoin_lst(user, JITOSOL::MINT)
});
router_instruction!(HYLOSOL, XSOL, LST_LOOKUP_TABLES, XSOL::MINT, |user| {
  account_builders::mint_levercoin_lst(user, HYLOSOL::MINT)
});

// `redeem_levercoin_lst`
router_instruction!(XSOL, JITOSOL, LST_LOOKUP_TABLES, JITOSOL::MINT, |user| {
  account_builders::redeem_levercoin_lst(user, JITOSOL::MINT)
});
router_instruction!(XSOL, HYLOSOL, LST_LOOKUP_TABLES, HYLOSOL::MINT, |user| {
  account_builders::redeem_levercoin_lst(user, HYLOSOL::MINT)
});

// `convert_stable_to_lever_lst`
router_instruction!(HYUSD, XSOL, BASE_LOOKUP_TABLES, XSOL::MINT, |user| {
  account_builders::convert_stable_to_lever_lst(user)
});

// `convert_lever_to_stable_lst`
router_instruction!(XSOL, HYUSD, BASE_LOOKUP_TABLES, HYUSD::MINT, |user| {
  account_builders::convert_lever_to_stable_lst(user)
});

// `swap_lst_to_lst`
router_instruction!(
  JITOSOL,
  HYLOSOL,
  LST_LOOKUP_TABLES,
  HYLOSOL::MINT,
  |user| account_builders::swap_lst_to_lst(user, JITOSOL::MINT, HYLOSOL::MINT,)
);
router_instruction!(
  HYLOSOL,
  JITOSOL,
  LST_LOOKUP_TABLES,
  JITOSOL::MINT,
  |user| account_builders::swap_lst_to_lst(user, HYLOSOL::MINT, JITOSOL::MINT,)
);

// `mint_stablecoin_usdc`
router_instruction!(USDC, HYUSD, BASE_LOOKUP_TABLES, HYUSD::MINT, |user| {
  account_builders::mint_stablecoin_usdc(user)
});

// `redeem_stablecoin_usdc`
router_instruction!(HYUSD, USDC, BASE_LOOKUP_TABLES, USDC::MINT, |user| {
  account_builders::redeem_stablecoin_usdc(user)
});

// `mint_stablecoin_exo`
router_instruction!(CBBTC, HYUSD, BASE_LOOKUP_TABLES, HYUSD::MINT, |user| {
  account_builders::mint_stablecoin_exo(
    user,
    CBBTC::MINT,
    pda::BTC_USD_PYTH_FEED,
  )
});

// `redeem_stablecoin_exo`
router_instruction!(HYUSD, CBBTC, BASE_LOOKUP_TABLES, CBBTC::MINT, |user| {
  account_builders::redeem_stablecoin_exo(
    user,
    CBBTC::MINT,
    pda::BTC_USD_PYTH_FEED,
  )
});

// `mint_levercoin_exo`
router_instruction!(
  CBBTC,
  XBTC,
  BASE_LOOKUP_TABLES,
  pda::exo_levercoin_mint(CBBTC::MINT),
  |user| account_builders::mint_levercoin_exo(
    user,
    CBBTC::MINT,
    pda::BTC_USD_PYTH_FEED,
  )
);

// `redeem_levercoin_exo`
router_instruction!(XBTC, CBBTC, BASE_LOOKUP_TABLES, CBBTC::MINT, |user| {
  account_builders::redeem_levercoin_exo(
    user,
    CBBTC::MINT,
    pda::BTC_USD_PYTH_FEED,
  )
});

// `convert_stable_to_lever_exo`
router_instruction!(
  HYUSD,
  XBTC,
  BASE_LOOKUP_TABLES,
  pda::exo_levercoin_mint(CBBTC::MINT),
  |user| account_builders::convert_stable_to_lever_exo(
    user,
    CBBTC::MINT,
    pda::BTC_USD_PYTH_FEED,
  )
);

// `convert_lever_to_stable_exo`
router_instruction!(XBTC, HYUSD, BASE_LOOKUP_TABLES, HYUSD::MINT, |user| {
  account_builders::convert_lever_to_stable_exo(
    user,
    CBBTC::MINT,
    pda::BTC_USD_PYTH_FEED,
  )
});

// `swap_lst_to_usdc`
router_instruction!(JITOSOL, USDC, LST_LOOKUP_TABLES, USDC::MINT, |user| {
  account_builders::swap_lst_to_usdc(user, JITOSOL::MINT, JITOSOL::POOL_STATE)
});
router_instruction!(HYLOSOL, USDC, LST_LOOKUP_TABLES, USDC::MINT, |user| {
  account_builders::swap_lst_to_usdc(user, HYLOSOL::MINT, HYLOSOL::POOL_STATE)
});

// `swap_usdc_to_lst`
router_instruction!(USDC, JITOSOL, LST_LOOKUP_TABLES, JITOSOL::MINT, |user| {
  account_builders::swap_usdc_to_lst(user, JITOSOL::MINT, JITOSOL::POOL_STATE)
});
router_instruction!(USDC, HYLOSOL, LST_LOOKUP_TABLES, HYLOSOL::MINT, |user| {
  account_builders::swap_usdc_to_lst(user, HYLOSOL::MINT, HYLOSOL::POOL_STATE)
});

// `swap_exo_to_usdc`
router_instruction!(CBBTC, USDC, BASE_LOOKUP_TABLES, USDC::MINT, |user| {
  account_builders::swap_exo_to_usdc(user, CBBTC::MINT, pda::BTC_USD_PYTH_FEED)
});

// `swap_usdc_to_exo`
router_instruction!(USDC, CBBTC, BASE_LOOKUP_TABLES, CBBTC::MINT, |user| {
  account_builders::swap_usdc_to_exo(user, CBBTC::MINT, pda::BTC_USD_PYTH_FEED)
});

// `user_deposit`
router_instruction!(HYUSD, SHYUSD, BASE_LOOKUP_TABLES, SHYUSD::MINT, |user| {
  ep_account_builders::deposit(user)
});

// `user_withdraw`
router_instruction!(SHYUSD, HYUSD, BASE_LOOKUP_TABLES, HYUSD::MINT, |user| {
  ep_account_builders::withdraw(user)
});
