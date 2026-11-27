use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anchor_client::solana_sdk::account::Account;
use anchor_client::solana_sdk::address_lookup_table::AddressLookupTableAccount;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::hash::Hash;
use anchor_client::solana_sdk::instruction::Instruction;
use anchor_client::solana_sdk::message::{v0, VersionedMessage};
use anchor_client::solana_sdk::signature::{Keypair, Signer};
use anchor_client::solana_sdk::transaction::VersionedTransaction;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::solana_program::sysvar::clock::{self, Clock};
use anchor_lang::{
  AccountDeserialize, AccountSerialize, AnchorDeserialize, Discriminator,
  InstructionData, Owner,
};
use anchor_spl::token::spl_token::state::Account as TokenAccount;
use anyhow::{anyhow, ensure, Context, Result};
use hylo_clients::util::{
  deserialize_lookup_table, HYLO_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE,
};
use hylo_idl::{earn_pool, exchange, pda, router};
use hylo_jupiter_amm_interface::{
  AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, SwapMode,
  SwapParams,
};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::{
  RpcSimulateTransactionAccountsConfig, RpcSimulateTransactionConfig,
};
use solana_rpc_client_api::response::RpcSimulateTransactionResult;
use solana_transaction_status_client_types::{
  UiInstruction, UiParsedInstruction,
};
use surfpool_sdk::cheatcodes::builders::{DeployProgram, SetAccount};
use surfpool_sdk::{
  BlockProductionMode, Keypair as SurfpoolKeypair, Pubkey as SurfpoolPubkey,
  Surfnet,
};

use crate::HyloJupiter;

const EVENT_CPI_PREFIX: [u8; 8] = 0x1d9a_cb51_2ea5_45e4_u64.to_le_bytes();
const PROGRAM_IDS: [Pubkey; 3] = [exchange::ID, earn_pool::ID, router::ID];

fn rpc_url() -> String {
  std::env::var("RPC_URL")
    .unwrap_or_else(|_| "https://api.mainnet.solana.com".to_string())
}

fn keypair_from_surfpool(keypair: &SurfpoolKeypair) -> Result<Keypair> {
  Ok(Keypair::try_from(keypair.to_bytes().as_slice())?)
}

fn pubkey_to_surfpool(pubkey: Pubkey) -> SurfpoolPubkey {
  SurfpoolPubkey::new_from_array(pubkey.to_bytes())
}

fn account_map(client: &RpcClient, keys: &[Pubkey]) -> Result<AccountMap> {
  Ok(
    keys
      .chunks(100)
      .map(|chunk| {
        client
          .get_multiple_accounts(chunk)?
          .into_iter()
          .zip(chunk.iter().copied())
          .map(|(account, key)| {
            account
              .map(|account| (key, account))
              .with_context(|| format!("missing required account {key}"))
          })
          .collect::<Result<Vec<_>>>()
      })
      .collect::<Result<Vec<_>>>()?
      .into_iter()
      .flatten()
      .collect(),
  )
}

fn validate_price_update_v2(account: &Account) -> bool {
  account.owner == PriceUpdateV2::owner()
    && account.data.starts_with(PriceUpdateV2::DISCRIMINATOR)
}

fn refresh_oracles(
  surfnet: &Surfnet,
  accounts: &mut AccountMap,
  clock: &Clock,
) -> Result<()> {
  let posted_slot =
    clock.slot.checked_sub(1).context("Surfpool slot is zero")?;
  let publish_time = clock
    .unix_timestamp
    .checked_sub(1)
    .context("Surfpool timestamp underflow")?;
  accounts
    .iter_mut()
    .filter(|(_, account)| validate_price_update_v2(account))
    .try_for_each(|(address, account)| -> Result<()> {
      let mut update =
        PriceUpdateV2::try_deserialize(&mut account.data.as_slice())?;
      update.posted_slot = posted_slot;
      update.price_message.publish_time = publish_time;
      update.try_serialize(&mut account.data.as_mut_slice())?;
      surfnet.cheatcodes().execute(
        SetAccount::new(pubkey_to_surfpool(*address))
          .data(account.data.clone()),
      )?;
      Ok(())
    })
}

fn set_clock(
  surfnet: &Surfnet,
  client: &RpcClient,
  clock: &Clock,
) -> Result<Clock> {
  let current: Clock =
    bincode::deserialize(&account_map(client, &[clock::ID])?[&clock::ID].data)?;
  let current: Clock = if clock.slot > current.slot {
    surfnet.cheatcodes().time_travel_to_slot(clock.slot)?;
    bincode::deserialize(&account_map(client, &[clock::ID])?[&clock::ID].data)?
  } else {
    current
  };
  if clock.unix_timestamp > current.unix_timestamp {
    let timestamp = u64::try_from(clock.unix_timestamp)?
      .checked_mul(1000)
      .context("Clock timestamp overflow")?;
    surfnet.cheatcodes().time_travel_to_timestamp(timestamp)?;
    Ok(bincode::deserialize(
      &account_map(client, &[clock::ID])?[&clock::ID].data,
    )?)
  } else {
    Ok(current)
  }
}

fn load_amm(surfnet: &Surfnet, client: &RpcClient) -> Result<HyloJupiter> {
  let accounts = account_map(client, &[pda::EXO_REGISTRY, clock::ID])?;
  let clock: Clock = bincode::deserialize(&accounts[&clock::ID].data)?;
  let context = AmmContext {
    clock_ref: clock.into(),
  };
  let upstream =
    RpcClient::new_with_commitment(rpc_url(), CommitmentConfig::confirmed());
  let mut amm = HyloJupiter::from_keyed_account(
    &KeyedAccount {
      key: pda::EXO_REGISTRY,
      account: accounts[&pda::EXO_REGISTRY].clone(),
      params: None,
    },
    &context,
  )?;
  // first iteration discovers exo pairs and derives oracle addresses
  // second iteration includes oracle accounts
  (0..2).try_for_each(|_| -> Result<()> {
    let mut accounts = account_map(client, &amm.get_accounts_to_update())?;
    let upstream_clock = upstream
      .get_account(&clock::ID)
      .map_err(|_| anyhow!("upstream Clock read failed"))?;
    let clock = set_clock(
      surfnet,
      client,
      &bincode::deserialize(&upstream_clock.data)?,
    )?;
    refresh_oracles(surfnet, &mut accounts, &clock)?;
    context.clock_ref.update(clock);
    amm.update(&accounts)
  })?;
  Ok(amm)
}

fn route(
  amm: &HyloJupiter,
  payer: Pubkey,
  params: &QuoteParams,
) -> Result<Instruction> {
  let swap = amm.get_swap_and_account_metas(&SwapParams {
    swap_mode: SwapMode::ExactIn,
    in_amount: params.amount,
    // 0 is placeholder
    out_amount: 0,
    source_mint: params.input_mint,
    destination_mint: params.output_mint,
    source_token_account: pda::ata(payer, params.input_mint),
    destination_token_account: pda::ata(payer, params.output_mint),
    token_transfer_authority: payer,
    quote_mint_to_referrer: None,
    jupiter_program_id: &router::ID,
    missing_dynamic_accounts_as_default: false,
  })?;
  Ok(Instruction {
    program_id: router::ID,
    accounts: swap.account_metas,
    data: router::client::args::RouteV2 {
      token_a: params.input_mint,
      token_b: params.output_mint,
      amount: params.amount,
      slippage_config: None,
    }
    .data(),
  })
}

fn transaction(
  payer: &Keypair,
  instruction: &Instruction,
  tables: &[AddressLookupTableAccount],
  blockhash: Hash,
) -> Result<VersionedTransaction> {
  let message = v0::Message::try_compile(
    &payer.pubkey(),
    std::slice::from_ref(instruction),
    tables,
    blockhash,
  )?;
  let transaction =
    VersionedTransaction::try_new(VersionedMessage::V0(message), &[payer])?;
  Ok(transaction)
}

fn check_quote<E: AnchorDeserialize + Discriminator>(
  surfnet: &Surfnet,
  params: &QuoteParams,
  emitter: Pubkey,
) -> Result<(E, Quote)> {
  let client = RpcClient::new(surfnet.rpc_url().to_owned());
  let amm = load_amm(surfnet, &client)?;
  let payer = keypair_from_surfpool(surfnet.payer())?;
  let instruction = route(&amm, payer.pubkey(), params)?;
  let table_keys = [HYLO_LOOKUP_TABLE, LST_REGISTRY_LOOKUP_TABLE];
  let table_accounts = account_map(&client, &table_keys)?;
  let tables = table_keys
    .into_iter()
    .map(|key| deserialize_lookup_table(&key, &table_accounts[&key]))
    .collect::<Result<Vec<_>>>()?;
  let balance = params
    .amount
    .checked_mul(2)
    .context("source balance overflow")?;
  [(params.input_mint, balance), (params.output_mint, 0)]
    .into_iter()
    .try_for_each(|(mint, amount)| {
      surfnet
        .cheatcodes()
        .fund_token(
          &pubkey_to_surfpool(payer.pubkey()),
          &pubkey_to_surfpool(mint),
          amount,
          None,
        )
        .map_err(|_| anyhow!("funding local token account for {mint}"))
    })?;
  let user_accounts = [
    pda::ata(payer.pubkey(), params.input_mint),
    pda::ata(payer.pubkey(), params.output_mint),
  ];
  let blockhash = client.get_latest_blockhash()?;
  let tx = transaction(&payer, &instruction, &tables, blockhash)?;
  let config = RpcSimulateTransactionConfig {
    sig_verify: false,
    inner_instructions: true,
    accounts: Some(RpcSimulateTransactionAccountsConfig {
      encoding: None,
      addresses: user_accounts.iter().map(ToString::to_string).collect(),
    }),
    ..RpcSimulateTransactionConfig::default()
  };
  let quote = amm.quote(params)?;
  let simulation = client
    .simulate_transaction_with_config(&tx, config.clone())?
    .value;
  assert_balances(&simulation, balance, &quote)?;
  Ok((extract_event(&simulation, emitter)?, quote))
}

fn assert_balances(
  simulation: &RpcSimulateTransactionResult,
  source_balance: u64,
  quote: &Quote,
) -> Result<()> {
  let post = simulation
    .accounts
    .as_ref()
    .context("missing post accounts")?;
  let amounts = post
    .iter()
    .map(|account| {
      let account: Account = account
        .as_ref()
        .and_then(anchor_client::solana_account_decoder::UiAccount::decode)
        .context("missing simulated token account")?;
      Ok(TokenAccount::unpack(&account.data)?.amount)
    })
    .collect::<Result<Vec<_>>>()?;
  ensure!(amounts.len() == 2, "expected two simulated token accounts");
  ensure!(
    source_balance.checked_sub(amounts[0]) == Some(quote.in_amount),
    "source debit differs from quote"
  );
  ensure!(
    amounts[1] == quote.out_amount,
    "destination credit differs from quote"
  );
  Ok(())
}

/// Compares a Jupiter quote with execution against Surfpool's account cache.
///
/// # Errors
/// * Local program loading, Surfpool startup, or account fetching fails.
/// * Quoting, execution, or parity assertions fail.
pub async fn quote_and_simulate<
  E: AnchorDeserialize + Discriminator + Send + 'static,
>(
  params: &QuoteParams,
  emitter: Pubkey,
) -> Result<(E, Quote)> {
  let params = QuoteParams {
    amount: params.amount,
    input_mint: params.input_mint,
    output_mint: params.output_mint,
    swap_mode: params.swap_mode,
  };
  let mut surfnet = load().await?;
  tokio::task::spawn_blocking(move || {
    let result = check_quote(&surfnet, &params, emitter).with_context(|| {
      format!(
        "route {} -> {}, amount {}",
        params.input_mint, params.output_mint, params.amount
      )
    });
    let shutdown = surfnet.stop();
    result.and_then(|value| {
      shutdown.context("stopping Surfpool")?;
      Ok(value)
    })
  })
  .await?
}

fn extract_event<E: AnchorDeserialize + Discriminator>(
  simulation: &RpcSimulateTransactionResult,
  emitter: Pubkey,
) -> Result<E> {
  let emitter = emitter.to_string();
  let instructions = simulation
    .inner_instructions
    .iter()
    .flatten()
    .flat_map(|inner| &inner.instructions)
    .filter_map(|instruction| match instruction {
      UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(
        instruction,
      )) if instruction.program_id == emitter => Some(&instruction.data),
      _ => None,
    })
    .map(|data| bs58::decode(data).into_vec())
    .collect::<Result<Vec<_>, _>>()?;
  let events: Vec<_> = instructions
    .iter()
    .filter_map(|bytes| bytes.strip_prefix(&EVENT_CPI_PREFIX))
    .filter_map(|bytes| bytes.strip_prefix(E::DISCRIMINATOR))
    .collect();
  ensure!(
    events.len() == 1,
    "expected one operation event from {emitter}, got {}",
    events.len()
  );
  Ok(E::try_from_slice(events[0])?)
}

fn program_path(program_id: Pubkey) -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("programs")
    .join(format!("{program_id}.so"))
}

fn program_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
  match fs::read(path) {
    Ok(bytes) => {
      ensure!(
        !bytes.is_empty(),
        "empty program binary: {}",
        path.display()
      );
      ensure!(
        bytes.starts_with(b"\x7fELF"),
        "invalid program binary: {}",
        path.display()
      );
      Ok(Some(bytes))
    }
    Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
    Err(error) => Err(error)
      .with_context(|| format!("reading program binary: {}", path.display())),
  }
}

fn deploy(surfnet: &Surfnet, id: Pubkey, bytes: Vec<u8>) -> Result<()> {
  surfnet
    .cheatcodes()
    .deploy(DeployProgram::new(pubkey_to_surfpool(id)).so_bytes(bytes))?;
  Ok(())
}

async fn load() -> Result<Surfnet> {
  let programs = PROGRAM_IDS
    .into_iter()
    .filter_map(|id| {
      program_bytes(&program_path(id))
        .map(|bytes| bytes.map(|bytes| (id, bytes)))
        .transpose()
    })
    .collect::<Result<Vec<_>>>()?;
  let surfnet = Surfnet::builder()
    .remote_rpc_url(rpc_url())
    .block_production_mode(BlockProductionMode::Manual)
    .start()
    .await?;
  programs.into_iter().try_for_each(|(id, bytes)| {
    deploy(&surfnet, id, bytes).with_context(|| {
      format!("loading program {id} from {}", program_path(id).display())
    })
  })?;
  Ok(surfnet)
}
