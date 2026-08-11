//! [`BuildTransactionData`] impls for [`RouterClient`].

use anyhow::Result;
use hylo_idl::tokens::{
  CBBTC, HYLOSOL, HYPE, HYUSD, JITOSOL, ONYC, PST, SHYUSD, USDC, WETH, XBTC,
  XETH, XHYPE, XONYC, XPST, XSOL, XZEC, ZEC,
};

use super::{InstructionBuilderExt, RouterArgs, RouterClient};
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::transaction::BuildTransactionData;

macro_rules! router_transaction_data {
  ($in:ty, $out:ty) => {
    #[async_trait::async_trait]
    impl BuildTransactionData<$in, $out> for RouterClient {
      type Inputs = RouterArgs;

      async fn build(
        &self,
        inputs: RouterArgs,
      ) -> Result<VersionedTransactionData> {
        let instructions =
          RouterClient::build_instructions::<$in, $out>(inputs)?;
        let lookup_tables = self
          .load_multiple_lookup_tables(
            RouterClient::lookup_tables::<$in, $out>(),
          )
          .await?;
        Ok(VersionedTransactionData::new(instructions, lookup_tables))
      }
    }
  };
}

// `mint_stablecoin_lst`
router_transaction_data!(JITOSOL, HYUSD);
router_transaction_data!(HYLOSOL, HYUSD);

// `redeem_stablecoin_lst`
router_transaction_data!(HYUSD, JITOSOL);
router_transaction_data!(HYUSD, HYLOSOL);

// `mint_levercoin_lst`
router_transaction_data!(JITOSOL, XSOL);
router_transaction_data!(HYLOSOL, XSOL);

// `redeem_levercoin_lst`
router_transaction_data!(XSOL, JITOSOL);
router_transaction_data!(XSOL, HYLOSOL);

// `convert_stable_to_lever_lst`
router_transaction_data!(HYUSD, XSOL);

// `convert_lever_to_stable_lst`
router_transaction_data!(XSOL, HYUSD);

// `swap_lst_to_lst`
router_transaction_data!(JITOSOL, HYLOSOL);
router_transaction_data!(HYLOSOL, JITOSOL);

// `mint_stablecoin_usdc`
router_transaction_data!(USDC, HYUSD);

// `redeem_stablecoin_usdc`
router_transaction_data!(HYUSD, USDC);

// `mint_stablecoin_exo`
router_transaction_data!(CBBTC, HYUSD);

// `redeem_stablecoin_exo`
router_transaction_data!(HYUSD, CBBTC);

// `mint_levercoin_exo`
router_transaction_data!(CBBTC, XBTC);

// `redeem_levercoin_exo`
router_transaction_data!(XBTC, CBBTC);

// `convert_stable_to_lever_exo`
router_transaction_data!(HYUSD, XBTC);

// `convert_lever_to_stable_exo`
router_transaction_data!(XBTC, HYUSD);

// `swap_lst_to_usdc`
router_transaction_data!(JITOSOL, USDC);
router_transaction_data!(HYLOSOL, USDC);

// `swap_usdc_to_lst`
router_transaction_data!(USDC, JITOSOL);
router_transaction_data!(USDC, HYLOSOL);

// `swap_exo_to_usdc`
router_transaction_data!(CBBTC, USDC);

// `swap_usdc_to_exo`
router_transaction_data!(USDC, CBBTC);

// `user_deposit`
router_transaction_data!(HYUSD, SHYUSD);

// `user_withdraw`
router_transaction_data!(SHYUSD, HYUSD);

router_transaction_data!(HYPE, HYUSD);
router_transaction_data!(HYUSD, HYPE);
router_transaction_data!(HYPE, XHYPE);
router_transaction_data!(XHYPE, HYPE);
router_transaction_data!(HYUSD, XHYPE);
router_transaction_data!(XHYPE, HYUSD);
router_transaction_data!(HYPE, USDC);
router_transaction_data!(USDC, HYPE);

router_transaction_data!(ZEC, HYUSD);
router_transaction_data!(HYUSD, ZEC);
router_transaction_data!(ZEC, XZEC);
router_transaction_data!(XZEC, ZEC);
router_transaction_data!(HYUSD, XZEC);
router_transaction_data!(XZEC, HYUSD);
router_transaction_data!(ZEC, USDC);
router_transaction_data!(USDC, ZEC);

router_transaction_data!(PST, HYUSD);
router_transaction_data!(HYUSD, PST);
router_transaction_data!(PST, XPST);
router_transaction_data!(XPST, PST);
router_transaction_data!(HYUSD, XPST);
router_transaction_data!(XPST, HYUSD);
router_transaction_data!(PST, USDC);
router_transaction_data!(USDC, PST);

router_transaction_data!(ONYC, HYUSD);
router_transaction_data!(HYUSD, ONYC);
router_transaction_data!(ONYC, XONYC);
router_transaction_data!(XONYC, ONYC);
router_transaction_data!(HYUSD, XONYC);
router_transaction_data!(XONYC, HYUSD);
router_transaction_data!(ONYC, USDC);
router_transaction_data!(USDC, ONYC);

router_transaction_data!(WETH, HYUSD);
router_transaction_data!(HYUSD, WETH);
router_transaction_data!(WETH, XETH);
router_transaction_data!(XETH, WETH);
router_transaction_data!(HYUSD, XETH);
router_transaction_data!(XETH, HYUSD);
router_transaction_data!(WETH, USDC);
router_transaction_data!(USDC, WETH);
