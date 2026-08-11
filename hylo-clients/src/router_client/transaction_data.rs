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

// `swap_lst_to_usdc`
router_transaction_data!(JITOSOL, USDC);
router_transaction_data!(HYLOSOL, USDC);

// `swap_usdc_to_lst`
router_transaction_data!(USDC, JITOSOL);
router_transaction_data!(USDC, HYLOSOL);

// `user_deposit`
router_transaction_data!(HYUSD, SHYUSD);

// `user_withdraw`
router_transaction_data!(SHYUSD, HYUSD);

macro_rules! exo_router_transaction_data {
  ($exo:ident, $lever:ident) => {
    router_transaction_data!($exo, HYUSD);
    router_transaction_data!(HYUSD, $exo);
    router_transaction_data!($exo, $lever);
    router_transaction_data!($lever, $exo);
    router_transaction_data!(HYUSD, $lever);
    router_transaction_data!($lever, HYUSD);
    router_transaction_data!($exo, USDC);
    router_transaction_data!(USDC, $exo);
  };
}

exo_router_transaction_data!(CBBTC, XBTC);
exo_router_transaction_data!(HYPE, XHYPE);
exo_router_transaction_data!(ZEC, XZEC);
exo_router_transaction_data!(PST, XPST);
exo_router_transaction_data!(ONYC, XONYC);
exo_router_transaction_data!(WETH, XETH);
