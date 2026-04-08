//! [`BuildTransactionData`] impls for [`RouterClient`].

use anyhow::Result;
use hylo_idl::tokens::{
  CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC, XSOL,
};

use super::RouterClient;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::syntax_helpers::InstructionBuilderExt;
use crate::transaction::{BuildTransactionData, RouterArgs};

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
