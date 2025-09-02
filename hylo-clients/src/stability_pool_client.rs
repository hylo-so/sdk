use crate::exchange_client::ExchangeClient;
use crate::program_client::{ProgramClient, VersionedTransactionData};
use crate::simulate_price::{RedeemArgs, HYUSD, JITOSOL, XSOL};
use crate::util::{
  parse_event, simulation_config, EXCHANGE_LOOKUP_TABLE,
  LST_REGISTRY_LOOKUP_TABLE, REFERENCE_WALLET, STABILITY_POOL_LOOKUP_TABLE,
};
use hylo_core::pyth::SOL_USD_PYTH_FEED;
use hylo_idl::exchange::events::{
  RedeemLevercoinEventV2, RedeemStablecoinEventV2,
};

use std::sync::Arc;

use anchor_client::solana_sdk::signature::{Keypair, Signature};
use anchor_client::Program;
use anchor_lang::prelude::Pubkey;
use anchor_lang::system_program;
use anchor_spl::{associated_token, token};
use anyhow::{anyhow, Result};
use fix::prelude::{UFix64, N6, *};
use hylo_idl::stability_pool::client::{accounts, args};
use hylo_idl::stability_pool::events::{
  StabilityPoolStats, UserDepositEvent, UserWithdrawEventV1,
};
use hylo_idl::{exchange, pda, stability_pool};

pub struct StabilityPoolClient {
  program: Program<Arc<Keypair>>,
  keypair: Arc<Keypair>,
}

impl ProgramClient for StabilityPoolClient {
  const PROGRAM_ID: Pubkey = stability_pool::ID;

  fn build_client(
    program: Program<Arc<Keypair>>,
    keypair: Arc<Keypair>,
  ) -> StabilityPoolClient {
    StabilityPoolClient { program, keypair }
  }

  fn program(&self) -> &Program<Arc<Keypair>> {
    &self.program
  }

  fn keypair(&self) -> Arc<Keypair> {
    self.keypair.clone()
  }
}

impl StabilityPoolClient {
  /// Creates transaction arguments for sHYUSD mint aka `user_deposit`.
  ///
  /// # Errors
  /// - Transaction building failure
  pub async fn mint_shyusd_args(
    &self,
    amount_hyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::UserDeposit {
      user,
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: pda::HYUSD,
      levercoin_mint: pda::XSOL,
      user_stablecoin_ata: pda::hyusd_ata(user),
      user_lp_token_ata: pda::shyusd_ata(user),
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      lp_token_auth: *pda::SHYUSD_AUTH,
      lp_token_mint: pda::SHYUSD,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::UserDeposit {
      amount_stablecoin: amount_hyusd.bits,
    };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData {
      instructions,
      lookup_tables,
    })
  }

  /// Mints sHYUSD LP tokens by depositing hyUSD.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn mint_shyusd(
    &self,
    amount_hyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<Signature> {
    let args = self.mint_shyusd_args(amount_hyusd, user).await?;
    let tx = self.build_v0_transaction(&args).await?;
    let sig = self
      .program()
      .rpc()
      .send_and_confirm_transaction(&tx)
      .await?;
    Ok(sig)
  }

  /// Creates transaction arguments for redeeming sHYUSD LP tokens.
  ///
  /// # Errors
  /// - Transaction building failure
  pub async fn redeem_shyusd_args(
    &self,
    amount_shyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let accounts = accounts::UserWithdraw {
      user,
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: pda::HYUSD,
      user_stablecoin_ata: pda::hyusd_ata(user),
      fee_auth: pda::fee_auth(pda::HYUSD),
      fee_vault: pda::fee_vault(pda::HYUSD),
      user_lp_token_ata: pda::shyusd_ata(user),
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_mint: pda::XSOL,
      levercoin_pool: *pda::XSOL_POOL,
      user_levercoin_ata: pda::xsol_ata(user),
      lp_token_auth: *pda::SHYUSD_AUTH,
      lp_token_mint: pda::SHYUSD,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      system_program: system_program::ID,
      token_program: token::ID,
      associated_token_program: associated_token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::UserWithdraw {
      amount_lp_token: amount_shyusd.bits,
    };
    let instructions = self
      .program()
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData {
      instructions,
      lookup_tables,
    })
  }

  /// Redeems sHYUSD LP tokens for hyUSD.
  ///
  /// # Errors
  /// - Failed to send transaction
  pub async fn redeem_shyusd(
    &self,
    amount_shyusd: UFix64<N6>,
    user: Pubkey,
  ) -> Result<Signature> {
    let args = self.redeem_shyusd_args(amount_shyusd, user).await?;
    let tx = self.build_v0_transaction(&args).await?;
    let sig = self
      .program()
      .rpc()
      .send_and_confirm_transaction(&tx)
      .await?;
    Ok(sig)
  }

  /// Build redeem transaction from sHYUSD directly to an LST via xSOL and
  /// hyUSD, useful for liquidation.
  ///
  /// # Errors
  /// - Transaction argument building
  /// - Simulation failures
  /// - Account loading
  pub async fn redeem_shyusd_lst_args(
    &self,
    exchange: &ExchangeClient,
    amount_shyusd: UFix64<N6>,
    user: Pubkey,
    lst_mint: Pubkey,
  ) -> Result<VersionedTransactionData> {
    let redeem_shyusd_args =
      self.redeem_shyusd_args(amount_shyusd, user).await?;
    let redeem_shyusd_tx = self
      .build_simulation_transaction(&user, &redeem_shyusd_args)
      .await?;
    let redeem_shyusd_sim = self
      .simulate_transaction_event::<UserWithdrawEventV1>(&redeem_shyusd_tx)
      .await?;
    let mut instructions = redeem_shyusd_args.instructions;
    if redeem_shyusd_sim.stablecoin_withdrawn.bits > 0 {
      let redeem_hyusd_args = exchange
        .build_transaction_data::<HYUSD, JITOSOL>(RedeemArgs {
          amount: UFix64::new(redeem_shyusd_sim.stablecoin_withdrawn.bits),
          lst_mint,
          user,
          slippage_config: None,
        })
        .await?;
      instructions.extend(redeem_hyusd_args.instructions);
    }
    if redeem_shyusd_sim.levercoin_withdrawn.bits > 0 {
      let redeem_xsol_args = exchange
        .build_transaction_data::<XSOL, JITOSOL>(RedeemArgs {
          amount: UFix64::new(redeem_shyusd_sim.levercoin_withdrawn.bits),
          lst_mint,
          user,
          slippage_config: None,
        })
        .await?;
      instructions.extend(redeem_xsol_args.instructions);
    }
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        LST_REGISTRY_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    Ok(VersionedTransactionData {
      instructions,
      lookup_tables,
    })
  }

  /// Execute redeem transaction from sHYUSD directly to an LST via xSOL and
  /// hyUSD, useful for liquidation.
  ///
  /// # Errors
  /// - Transaction building
  /// - Transaction sending and confirmation
  pub async fn redeem_shyusd_lst(
    &self,
    exchange: &ExchangeClient,
    amount_shyusd: UFix64<N6>,
    user: Pubkey,
    lst_mint: Pubkey,
  ) -> Result<Signature> {
    let args = self
      .redeem_shyusd_lst_args(exchange, amount_shyusd, user, lst_mint)
      .await?;
    let tx = self.build_v0_transaction(&args).await?;
    let sig = self
      .program()
      .rpc()
      .send_and_confirm_transaction(&tx)
      .await?;
    Ok(sig)
  }

  /// Rebalances stability pool by swapping stablecoin to levercoin.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn rebalance_stable_to_lever(&self) -> Result<Signature> {
    let accounts = accounts::RebalanceStableToLever {
      payer: self.program.payer(),
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: pda::HYUSD,
      stablecoin_pool: *pda::HYUSD_POOL,
      pool_auth: *pda::POOL_AUTH,
      levercoin_pool: *pda::XSOL_POOL,
      fee_auth: pda::fee_auth(pda::HYUSD),
      fee_vault: pda::fee_vault(pda::HYUSD),
      levercoin_mint: pda::XSOL,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      token_program: token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::RebalanceStableToLever {};
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    let tx_args = VersionedTransactionData {
      instructions,
      lookup_tables,
    };
    let sig = self.send_v0_transaction(&tx_args).await?;
    Ok(sig)
  }

  /// Rebalances levercoin from the stability pool back to stablecoin.
  ///
  /// # Errors
  /// - Transaction failure
  pub async fn rebalance_lever_to_stable(&self) -> Result<Signature> {
    let accounts = accounts::RebalanceLeverToStable {
      payer: self.program.payer(),
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: pda::HYUSD,
      stablecoin_pool: *pda::HYUSD_POOL,
      pool_auth: *pda::POOL_AUTH,
      levercoin_pool: *pda::XSOL_POOL,
      fee_auth: pda::fee_auth(pda::HYUSD),
      fee_vault: pda::fee_vault(pda::HYUSD),
      levercoin_mint: pda::XSOL,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
      stablecoin_auth: *pda::HYUSD_AUTH,
      levercoin_auth: *pda::XSOL_AUTH,
      hylo_event_authority: *pda::EXCHANGE_EVENT_AUTH,
      hylo_exchange_program: exchange::ID,
      token_program: token::ID,
      event_authority: *pda::STABILITY_POOL_EVENT_AUTH,
      program: stability_pool::ID,
    };
    let args = args::RebalanceLeverToStable {};
    let instructions = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .instructions()?;
    let lookup_tables = self
      .load_multiple_lookup_tables(&[
        EXCHANGE_LOOKUP_TABLE,
        STABILITY_POOL_LOOKUP_TABLE,
      ])
      .await?;
    let tx_args = VersionedTransactionData {
      instructions,
      lookup_tables,
    };
    let sig = self.send_v0_transaction(&tx_args).await?;
    Ok(sig)
  }

  /// Simulates the `get_stats` instruction on the stability pool.
  ///
  /// # Errors
  /// - Simulation failure
  /// - Return data access or deserialization
  pub async fn get_stats(&self) -> Result<StabilityPoolStats> {
    let accounts = accounts::GetStats {
      pool_config: *pda::POOL_CONFIG,
      hylo: *pda::HYLO,
      stablecoin_mint: pda::HYUSD,
      levercoin_mint: pda::XSOL,
      pool_auth: *pda::POOL_AUTH,
      stablecoin_pool: *pda::HYUSD_POOL,
      levercoin_pool: *pda::XSOL_POOL,
      lp_token_mint: pda::SHYUSD,
      sol_usd_pyth_feed: SOL_USD_PYTH_FEED,
    };
    let args = args::GetStats {};
    let tx = self
      .program
      .request()
      .accounts(accounts)
      .args(args)
      .signed_transaction()
      .await?;
    let stats = self.simulate_transaction_return(tx.into()).await?;
    Ok(stats)
  }

  /// Quotes minting of sHYUSD for 1 unit of hyUSD via simulation.
  ///
  /// # Errors
  /// - Transaction simulation
  /// - Event parsing
  pub async fn quote_shyusd_mint(&self) -> Result<UFix64<N6>> {
    let args = self
      .mint_shyusd_args(UFix64::one(), REFERENCE_WALLET)
      .await?;
    let tx = self
      .build_simulation_transaction(&REFERENCE_WALLET, &args)
      .await?;
    let event = self
      .simulate_transaction_event::<UserDepositEvent>(&tx)
      .await?;
    Ok(UFix64::new(event.lp_token_minted.bits))
  }

  /// Quotes redemption to hyUSD for 1 unit of sHYUSD via simulation.
  ///
  /// # Errors
  /// - Transaction simulation
  /// - Event parsing
  /// - Levercoin present in pool
  pub async fn quote_shyusd_redeem(&self) -> Result<UFix64<N6>> {
    let args = self
      .redeem_shyusd_args(UFix64::one(), REFERENCE_WALLET)
      .await?;
    let tx = self
      .build_simulation_transaction(&REFERENCE_WALLET, &args)
      .await?;
    let event = self
      .simulate_transaction_event::<UserWithdrawEventV1>(&tx)
      .await?;
    if event.levercoin_withdrawn.bits > 0 {
      return Err(anyhow!(
        "Cannot quote sHYUSD/hyUSD: levercoin present in pool"
      ));
    }
    Ok(UFix64::new(event.stablecoin_withdrawn.bits))
  }

  /// Quotes redemption to LST for 1 unit of sHYUSD via simulation.
  ///
  /// # Errors
  /// - Transaction simulation
  /// - Event parsing
  pub async fn quote_shyusd_redeem_lst(
    &self,
    exchange: &ExchangeClient,
    lst_mint: Pubkey,
  ) -> Result<UFix64<N9>> {
    let args = self
      .redeem_shyusd_lst_args(
        exchange,
        UFix64::one(),
        REFERENCE_WALLET,
        lst_mint,
      )
      .await?;
    let tx = self
      .build_simulation_transaction(&REFERENCE_WALLET, &args)
      .await?;
    let rpc = self.program().rpc();
    let sim_result = rpc
      .simulate_transaction_with_config(&tx, simulation_config())
      .await?;
    let from_xsol = parse_event::<RedeemLevercoinEventV2>(&sim_result)
      .map_or(UFix64::zero(), |e| {
        UFix64::<N9>::new(e.collateral_withdrawn.bits)
      });
    let from_hyusd = parse_event::<RedeemStablecoinEventV2>(&sim_result)
      .map_or(UFix64::zero(), |e| {
        UFix64::<N9>::new(e.collateral_withdrawn.bits)
      });
    let total_out = from_hyusd
      .checked_add(&from_xsol)
      .ok_or(anyhow!("total_out overflow"))?;
    Ok(total_out)
  }
}

#[cfg(test)]
mod tests {
  use crate::util::{
    build_test_exchange_client, build_test_stability_pool_client,
  };

  use anchor_lang::solana_program::pubkey;
  use anyhow::Result;

  #[tokio::test]
  async fn print_quote() -> Result<()> {
    let jitosol = pubkey!("J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn");
    let client = build_test_stability_pool_client()?;
    let exchange = build_test_exchange_client()?;
    let quote = client.quote_shyusd_redeem_lst(&exchange, jitosol).await?;
    println!("{quote:?}");
    Ok(())
  }
}
