//! [`InstructionBuilder`] impls for [`RouterClient`](super::RouterClient).

use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_lang::ToAccountMetas;
use anyhow::Result;
use hylo_core::pyth::PythOracle;
use hylo_core::slippage_config::SlippageConfig;
use hylo_idl::earn_pool::account_builders as ep_account_builders;
use hylo_idl::exchange::account_builders;
use hylo_idl::router::client::args as router_args;
use hylo_idl::router::instruction_builders::route;
use hylo_idl::tokens::{
  StakePool, TokenMint, CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, ONYC, PST,
  SHYUSD, USDC, WETH, XBTC, XETH, XHYPE, XONYC, XPST, XSOL, XZEC, ZEC,
};
use hylo_idl::with_exo_pairs;

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

// `user_deposit`
router_instruction!(HYUSD, SHYUSD, BASE_LOOKUP_TABLES, SHYUSD::MINT, |user| {
  ep_account_builders::deposit(user)
});

// `user_withdraw`
router_instruction!(SHYUSD, HYUSD, BASE_LOOKUP_TABLES, HYUSD::MINT, |user| {
  ep_account_builders::withdraw(user)
});

macro_rules! exo_router_instructions {
  ($(($exo:ident, $lever:ident, $exp:ty)),+ $(,)?) => {
    $(
    router_instruction!($exo, HYUSD, BASE_LOOKUP_TABLES, HYUSD::MINT, |user| {
      account_builders::mint_stablecoin_exo(
        user,
        $exo::MINT,
        $exo::FEED.address,
      )
    });

    router_instruction!(
      $exo,
      $lever,
      BASE_LOOKUP_TABLES,
      $lever::MINT,
      |user| {
        account_builders::mint_levercoin_exo(
          user,
          $exo::MINT,
          $exo::FEED.address,
        )
      }
    );

    router_instruction!(HYUSD, $exo, BASE_LOOKUP_TABLES, $exo::MINT, |user| {
      account_builders::redeem_stablecoin_exo(
        user,
        $exo::MINT,
        $exo::FEED.address,
      )
    });

    router_instruction!($lever, $exo, BASE_LOOKUP_TABLES, $exo::MINT, |user| {
      account_builders::redeem_levercoin_exo(
        user,
        $exo::MINT,
        $exo::FEED.address,
      )
    });

    router_instruction!(
      HYUSD,
      $lever,
      BASE_LOOKUP_TABLES,
      $lever::MINT,
      |user| {
        account_builders::convert_stable_to_lever_exo(
          user,
          $exo::MINT,
          $exo::FEED.address,
        )
      }
    );

    router_instruction!(
      $lever,
      HYUSD,
      BASE_LOOKUP_TABLES,
      HYUSD::MINT,
      |user| {
        account_builders::convert_lever_to_stable_exo(
          user,
          $exo::MINT,
          $exo::FEED.address,
        )
      }
    );

    router_instruction!($exo, USDC, BASE_LOOKUP_TABLES, USDC::MINT, |user| {
      account_builders::swap_exo_to_usdc(user, $exo::MINT, $exo::FEED.address)
    });

    router_instruction!(USDC, $exo, BASE_LOOKUP_TABLES, $exo::MINT, |user| {
      account_builders::swap_usdc_to_exo(user, $exo::MINT, $exo::FEED.address)
    });
    )+
  };
}

with_exo_pairs!(exo_router_instructions);
