# Crate: hylo_clients (lib)

# crate
<!-- file: hylo-clients/src/lib.rs -->

---

# crate::exchange_client
<!-- file: hylo-clients/src/exchange_client.rs -->

## Types

/// Admin client for the Hylo exchange program. Manages LST
/// registration, oracle configuration, fee updates, and protocol
/// stats. User-facing operations go through [`RouterClient`].
// NOTE: Anchor-based RPC client for the Hylo Exchange program, supporting mint/redeem/swap and admin operations.
pub struct ExchangeClient {
    program: Program < Arc < Keypair > >,
    keypair: Arc < Keypair >,
}


## Impl ProgramClient for ExchangeClient

// NOTE: Implements ProgramClient providing the exchange program ID and client construction.
impl ProgramClient for ExchangeClient {
    const PROGRAM_ID: Pubkey;
    fn build_client(program : Program < Arc < Keypair > >, keypair : Arc < Keypair >) -> ExchangeClient;
    fn program(& self) -> & Program < Arc < Keypair > >;
    fn keypair(& self) -> Arc < Keypair >;
}


## Impl ExchangeClient

// NOTE: Anchor-based RPC client for the Hylo Exchange program, supporting mint/redeem/swap and admin operations.
impl ExchangeClient {
    pub fn initialize_protocol(& self, upgrade_authority : Pubkey, treasury : Pubkey, args : & args :: InitializeProtocol) -> Result < VersionedTransactionData >;
    pub fn initialize_mints(& self, stablecoin_metadata : TokenMetadata, levercoin_metadata : TokenMetadata) -> Result < VersionedTransactionData >;
    pub fn initialize_lst_registry(& self, slot : u64) -> Result < VersionedTransactionData >;
    pub fn initialize_lst_registry_calculators(& self, lst_registry : Pubkey) -> Result < VersionedTransactionData >;
    pub fn register_lst(& self, lst_registry : Pubkey, lst_mint : Pubkey, lst_stake_pool_state : Pubkey, sanctum_calculator_program : Pubkey, sanctum_calculator_state : Pubkey, stake_pool_program : Pubkey, stake_pool_program_data : Pubkey, rebalance_fee : UFixValue64) -> Result < VersionedTransactionData >;
    pub async fn update_lst_prices(& self) -> Result < VersionedTransactionData >;
    pub async fn harvest_yield(& self) -> Result < VersionedTransactionData >;
    pub fn update_oracle_conf_tolerance(& self, args : & args :: UpdateOracleConfTolerance) -> Result < VersionedTransactionData >;
    pub fn update_sol_usd_oracle(& self, args : & args :: UpdateSolUsdOracle) -> Result < VersionedTransactionData >;
    pub fn update_stability_pool(& self, args : & args :: UpdateStabilityPool) -> Result < VersionedTransactionData >;
    pub fn update_lst_swap_fee(& self, args : & args :: UpdateLstSwapFee) -> Result < VersionedTransactionData >;
    pub fn update_levercoin_fees(& self, args : & args :: UpdateLevercoinFees) -> Result < VersionedTransactionData >;
    pub fn update_oracle_interval(& self, args : & args :: UpdateOracleInterval) -> Result < VersionedTransactionData >;
    pub fn update_stability_thresholds(& self, args : & args :: UpdateStabilityThresholds) -> Result < VersionedTransactionData >;
    pub fn update_paused(& self, args : & args :: UpdatePaused) -> Result < VersionedTransactionData >;
    pub fn update_lst_buy_curve_config(& self, args : & args :: UpdateLstBuyCurveConfig) -> Result < VersionedTransactionData >;
    pub fn update_lst_sell_curve_config(& self, args : & args :: UpdateLstSellCurveConfig) -> Result < VersionedTransactionData >;
    pub fn update_treasury(& self, args : & args :: UpdateTreasury) -> Result < VersionedTransactionData >;
    pub fn update_yield_harvest_config(& self, args : & args :: UpdateYieldHarvestConfig) -> Result < VersionedTransactionData >;
    pub fn update_usdc_oracle_conf_tolerance(& self, args : & args :: UpdateUsdcOracleConfTolerance) -> Result < VersionedTransactionData >;
    pub fn update_usdc_oracle_interval(& self, args : & args :: UpdateUsdcOracleInterval) -> Result < VersionedTransactionData >;
    pub fn update_usdc_swap_fee(& self, args : & args :: UpdateUsdcSwapFee) -> Result < VersionedTransactionData >;
    pub fn update_admin(& self, upgrade_authority : Pubkey, args : & args :: UpdateAdmin) -> Result < VersionedTransactionData >;
    pub fn update_lst_rebalance_fee(& self, lst_mint : Pubkey, args : & args :: UpdateLstRebalanceFee) -> Result < VersionedTransactionData >;
    pub fn update_exo_funding_rate(& self, collateral_mint : Pubkey, args : & args :: UpdateExoFundingRate) -> Result < VersionedTransactionData >;
    pub fn update_exo_oracle(& self, collateral_mint : Pubkey, args : & args :: UpdateExoOracle) -> Result < VersionedTransactionData >;
    pub fn update_exo_oracle_conf_tolerance(& self, collateral_mint : Pubkey, args : & args :: UpdateExoOracleConfTolerance) -> Result < VersionedTransactionData >;
    pub fn update_exo_oracle_interval(& self, collateral_mint : Pubkey, args : & args :: UpdateExoOracleInterval) -> Result < VersionedTransactionData >;
    pub fn update_exo_stability_threshold(& self, collateral_mint : Pubkey, args : & args :: UpdateExoStabilityThreshold) -> Result < VersionedTransactionData >;
    pub fn update_exo_buy_curve(& self, collateral_mint : Pubkey, args : & args :: UpdateExoBuyCurve) -> Result < VersionedTransactionData >;
    pub fn update_exo_sell_curve(& self, collateral_mint : Pubkey, args : & args :: UpdateExoSellCurve) -> Result < VersionedTransactionData >;
    pub fn update_exo_levercoin_fees(& self, collateral_mint : Pubkey, args : & args :: UpdateExoLevercoinFees) -> Result < VersionedTransactionData >;
    pub fn initialize_usdc(& self, usdc_usd_pyth_feed : Pubkey, args : & args :: InitializeUsdc) -> Result < VersionedTransactionData >;
    pub fn initialize_lst_virtual_stablecoin(& self) -> Result < VersionedTransactionData >;
    pub fn register_exo(& self, collateral_mint : Pubkey, exo_usd_pyth_feed : Pubkey, args : & args :: RegisterExo) -> Result < VersionedTransactionData >;
    pub fn withdraw_fees(& self, treasury : Pubkey, fee_token_mint : Pubkey) -> Result < VersionedTransactionData >;
    pub fn harvest_funding_rate(& self, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> Result < VersionedTransactionData >;
}


---

# crate::prelude
<!-- file: hylo-clients/src/prelude.rs -->

## Re-exports

// NOTE: Re-export of Solana RPC commitment level configuration.
pub use anchor_client :: solana_sdk :: commitment_config :: CommitmentConfig;

// NOTE: Re-export of Solana transaction signature type.
pub use anchor_client :: solana_sdk :: signature :: Signature;

// NOTE: Re-export of Solana cluster configuration (Mainnet, Devnet, custom).
pub use anchor_client :: Cluster;

// NOTE: Re-export of Solana public key type.
pub use anchor_lang :: prelude :: Pubkey;

// NOTE: Re-export of anyhow::Result for ergonomic error handling.
pub use anyhow :: Result;

// NOTE: Glob re-export of hylo-fix fixed-point math prelude (UFix64, IFix64, exponents).
pub use fix :: prelude :: *;

// NOTE: Re-export of token type definitions from hylo-core's IDL.
pub use hylo_core :: idl :: tokens :: { HYUSD , JITOSOL , SHYUSD , XSOL };

// NOTE: Re-export of ExchangeClient for convenient imports.
pub use crate :: exchange_client :: ExchangeClient;

// NOTE: Re-export of ProgramClient trait and VersionedTransactionData.
pub use crate :: program_client :: { ProgramClient , VersionedTransactionData };

pub use crate :: router_client :: { InstructionBuilder , InstructionBuilderExt , RouterArgs , RouterClient , };

// NOTE: Re-export of StabilityPoolClient for convenient imports.
pub use crate :: stability_pool_client :: StabilityPoolClient;

// NOTE: Re-export of transaction argument types and traits.
pub use crate :: transaction :: { BuildTransactionData , TransactionSyntax };


---

# crate::program_client
<!-- file: hylo-clients/src/program_client.rs -->

## Types

/// Components from which a [`VersionedTransaction`] can be built.
// NOTE: Instructions and lookup tables from which a VersionedTransaction can be assembled.
pub struct VersionedTransactionData {
    pub instructions: Vec < Instruction >,
    pub lookup_tables: Vec < AddressLookupTableAccount >,
}


## Traits

/// Abstracts the construction of client structs with `anchor_client::Program`.
// NOTE: Base trait abstracting Anchor program client construction, transaction building, simulation, and RPC operations.
pub trait ProgramClient: Sized {
    const PROGRAM_ID: Pubkey;
    fn build_client(program : Program < Arc < Keypair > >, keypair : Arc < Keypair >) -> Self;
    fn program(& self) -> & Program < Arc < Keypair > >;
    fn keypair(& self) -> Arc < Keypair >;
    fn new_from_keypair(cluster : Cluster, keypair : Keypair, config : CommitmentConfig) -> Result < Self >;
    fn new_random_keypair(cluster : Cluster, config : CommitmentConfig) -> Result < Self >;
    async fn build_v0_transaction(& self, vtd : & VersionedTransactionData) -> Result < VersionedTransaction >;
    async fn build_simulation_transaction(& self, for_user : & Pubkey, VersionedTransactionData { instructions , lookup_tables , } : & VersionedTransactionData) -> Result < VersionedTransaction >;
    async fn send_v0_transaction(& self, args : & VersionedTransactionData) -> Result < Signature >;
    async fn load_lst_registry(& self) -> Result < (Vec < AccountMeta > , AddressLookupTableAccount) >;
    async fn load_lookup_table(& self, key : & Pubkey) -> Result < AddressLookupTableAccount >;
    async fn load_multiple_lookup_tables(& self, pubkeys : & [Pubkey]) -> Result < Vec < AddressLookupTableAccount > >;
    async fn simulate_transaction_return< R : AnchorDeserialize >(& self, tx : & VersionedTransaction) -> Result < R >;
    async fn simulate_transaction_return_with_cus< R : AnchorDeserialize >(& self, tx : & VersionedTransaction) -> Result < (R , Option < u64 >) >;
    async fn simulate_transaction_event< E : AnchorDeserialize + Discriminator >(& self, tx : & VersionedTransaction) -> Result < E >;
    async fn simulate_transaction_event_with_cus< E : AnchorDeserialize + Discriminator , >(& self, tx : & VersionedTransaction) -> Result < (E , Option < u64 >) >;
}


## Impl VersionedTransactionData

// NOTE: Instructions and lookup tables from which a VersionedTransaction can be assembled.
impl VersionedTransactionData {
    pub fn one(instruction : Instruction) -> VersionedTransactionData;
    pub fn new(instructions : Vec < Instruction >, lookup_tables : Vec < AddressLookupTableAccount >) -> VersionedTransactionData;
}


---

# crate::router_client
<!-- file: hylo-clients/src/router_client/mod.rs -->

## Types

/// Arguments for all router-based token operations.
pub struct RouterArgs {
    pub amount: u64,
    pub user: Pubkey,
    pub slippage_config: Option < SlippageConfig >,
}

/// Builds and executes transactions through the Hylo router program.
/// Handles all user-facing token operations: mint, redeem, swap, and
/// stability pool deposit/withdraw.
pub struct RouterClient {
    program: Program < Arc < Keypair > >,
    keypair: Arc < Keypair >,
}


## Traits

pub trait InstructionBuilder< IN : TokenMint , OUT : TokenMint > {
    type Inputs;
    const REQUIRED_LOOKUP_TABLES: & 'static [Pubkey];
    fn build(inputs : Self :: Inputs) -> Result < Vec < Instruction > >;
}

pub trait InstructionBuilderExt {
    fn build_instructions< IN , OUT >(inputs : < Self as InstructionBuilder < IN , OUT > > :: Inputs) -> Result < Vec < Instruction > >;
    fn lookup_tables< IN , OUT >() -> & 'static [Pubkey];
}


## Impl InstructionBuilderExt for X

impl < X >InstructionBuilderExt for X {
    fn build_instructions< IN , OUT >(inputs : < Self as InstructionBuilder < IN , OUT > > :: Inputs) -> Result < Vec < Instruction > >;
    fn lookup_tables< IN , OUT >() -> & 'static [Pubkey];
}


## Impl ProgramClient for RouterClient

impl ProgramClient for RouterClient {
    const PROGRAM_ID: Pubkey;
    fn build_client(program : Program < Arc < Keypair > >, keypair : Arc < Keypair >) -> RouterClient;
    fn program(& self) -> & Program < Arc < Keypair > >;
    fn keypair(& self) -> Arc < Keypair >;
}


## Impl TransactionSyntax for RouterClient

impl TransactionSyntax for RouterClient {

}


---

# crate::router_client::instructions
<!-- file: hylo-clients/src/router_client/instructions.rs -->

## Functions

fn route_instruction< A : ToAccountMetas >(token_a : Pubkey, token_b : Pubkey, amount : u64, slippage_config : Option < SlippageConfig >, inner_accounts : & A) -> Instruction;


## Constants

const BASE_LOOKUP_TABLES: & [Pubkey];

const LST_LOOKUP_TABLES: & [Pubkey];


## Macros

macro_rules! router_instruction { ... }


---

# crate::router_client::transaction_data
<!-- file: hylo-clients/src/router_client/transaction_data.rs -->

## Macros

macro_rules! router_transaction_data { ... }


---

# crate::stability_pool_client
<!-- file: hylo-clients/src/stability_pool_client.rs -->

## Types

/// Admin client for the Hylo stability pool program. Manages pool
/// initialization, rebalancing, fee configuration, and stats.
/// User-facing deposit/withdraw goes through [`RouterClient`].
// NOTE: Anchor-based RPC client for the Hylo Stability Pool program, supporting deposit, withdraw, and rebalance.
pub struct StabilityPoolClient {
    program: Program < Arc < Keypair > >,
    keypair: Arc < Keypair >,
}


## Impl ProgramClient for StabilityPoolClient

// NOTE: Implements ProgramClient providing the stability pool program ID and client construction.
impl ProgramClient for StabilityPoolClient {
    const PROGRAM_ID: Pubkey;
    fn build_client(program : Program < Arc < Keypair > >, keypair : Arc < Keypair >) -> StabilityPoolClient;
    fn program(& self) -> & Program < Arc < Keypair > >;
    fn keypair(& self) -> Arc < Keypair >;
}


## Impl StabilityPoolClient

// NOTE: Anchor-based RPC client for the Hylo Stability Pool program, supporting deposit, withdraw, and rebalance.
impl StabilityPoolClient {
    pub async fn rebalance_lever_to_stable(& self) -> Result < Signature >;
    pub fn initialize_stability_pool(& self, upgrade_authority : Pubkey) -> Result < VersionedTransactionData >;
    pub fn initialize_lp_token_mint(& self, lp_token_metadata : TokenMetadata) -> Result < VersionedTransactionData >;
    pub fn update_withdrawal_fee(& self, args : & args :: UpdateWithdrawalFee) -> Result < VersionedTransactionData >;
    pub fn update_admin(& self, upgrade_authority : Pubkey, args : & args :: UpdateAdmin) -> Result < VersionedTransactionData >;
}


---

# crate::transaction
<!-- file: hylo-clients/src/transaction.rs -->

## Traits

/// Builds transaction data for a token pair operation.
// NOTE: Async trait for building VersionedTransactionData from typed inputs for a token pair.
pub trait BuildTransactionData< I , O > {
    type Inputs: Send + Sync + 'static;
    async fn build(& self, inputs : Self :: Inputs) -> Result < VersionedTransactionData >;
}

/// High-level API for transaction operations.
// NOTE: High-level trait providing run_transaction, build_transaction_data, and simulate_event methods.
pub trait TransactionSyntax {
    async fn run_transaction< I , O >(& self, inputs : < Self as BuildTransactionData < I , O > > :: Inputs) -> Result < Signature >;
    async fn build_transaction_data< I , O >(& self, inputs : < Self as BuildTransactionData < I , O > > :: Inputs) -> Result < VersionedTransactionData >;
    async fn simulate_event< I , O , E >(& self, user : Pubkey, inputs : < Self as BuildTransactionData < I , O > > :: Inputs) -> Result < E >;
    async fn simulate_event_with_cus< I , O , E >(& self, user : Pubkey, inputs : < Self as BuildTransactionData < I , O > > :: Inputs) -> Result < (E , Option < u64 >) >;
}


---

# crate::util
<!-- file: hylo-clients/src/util.rs -->

## Traits

// NOTE: Marker trait for liquid staking tokens, requiring N9 decimal precision.
pub trait LST: StakePool {

}


## Functions

/// Default configuration to use in simulated transactions.
// NOTE: Returns default RpcSimulateTransactionConfig for transaction simulations.
pub fn simulation_config() -> RpcSimulateTransactionConfig;

/// Parses a typed event from simulation inner instructions.
/// 
/// # Errors
/// - Simulation failed
/// - No inner instructions returned
/// - Event not found or deserialization fails
pub fn parse_event< E >(result : & Response < RpcSimulateTransactionResult >) -> Result < E > where E : AnchorDeserialize + Discriminator ,;

/// Deserializes an account into an address lookup table.
/// 
/// # Errors
/// - Account data cannot be deserialized
// NOTE: Deserializes a Solana account into an AddressLookupTableAccount.
pub fn deserialize_lookup_table(key : & Pubkey, account : & Account) -> Result < AddressLookupTableAccount >;

/// Builds a signed versioned transaction.
/// 
/// # Errors
/// - Failed to compile message
/// - Failed to create transaction
// NOTE: Assembles a signed VersionedTransaction from instructions, lookup tables, and signers.
pub fn build_v0_transaction(VersionedTransactionData { instructions , lookup_tables , } : & VersionedTransactionData, payer : & Keypair, additional_signers : & [& Keypair], recent_blockhash : Hash) -> Result < VersionedTransaction >;

/// Creates `remaining_accounts` array from LST registry table with all
/// headers writable.
/// 
/// # Errors
/// - Lookup table account doesn't exist
/// - Malformed structure (preamble cannot be split at 16)
// NOTE: Constructs remaining_accounts and lookup table from the LST registry for instruction building.
pub fn build_lst_registry(table : AddressLookupTableAccount) -> Result < (Vec < AccountMeta > , AddressLookupTableAccount) >;

/// Gets cluster from environment variables.
/// 
/// # Errors
/// - Missing `RPC_URL` or `RPC_WS_URL` environment variables
// NOTE: Reads RPC_URL and RPC_WS_URL environment variables to construct a Cluster.
pub fn cluster_from_env() -> Result < Cluster >;

/// Builds test exchange client with random keypair.
/// 
/// # Errors
/// - Environment variable access
/// - Client initialization
// NOTE: Creates an ExchangeClient with random keypair from env vars, for integration tests.
pub fn build_test_exchange_client() -> Result < ExchangeClient >;

/// Builds test stability pool client with random keypair.
/// 
/// # Errors
/// - Environment variable access
/// - Client initialization
// NOTE: Creates a StabilityPoolClient with random keypair from env vars, for integration tests.
pub fn build_test_stability_pool_client() -> Result < StabilityPoolClient >;

/// Builds test router client with random keypair.
/// 
/// # Errors
/// - Environment variable access
/// - Client initialization
pub fn build_test_router_client() -> Result < RouterClient >;

/// Builds ATA creation instruction for a user and mint.
// NOTE: Creates a create-associated-token-account instruction for a user and mint.
pub fn user_ata_instruction(user : & Pubkey, mint : & Pubkey) -> Instruction;


## Impl LST for JITOSOL

// NOTE: Marks JITOSOL as a valid LST collateral type.
impl LST for JITOSOL {

}


## Impl LST for HYLOSOL

// NOTE: Marks HYLOSOL as a valid LST collateral type.
impl LST for HYLOSOL {

}


## Constants

pub const HYLO_LOOKUP_TABLE: Pubkey;

pub const HYLO_LOOKUP_TABLE: Pubkey;

// NOTE: Pubkey of the address lookup table for LST registry accounts.
pub const LST_REGISTRY_LOOKUP_TABLE: Pubkey;

// NOTE: Pubkey of the address lookup table for LST registry accounts.
pub const LST_REGISTRY_LOOKUP_TABLE: Pubkey;

/// This wallet should hold at least one unit of jitoSOL, xSOL, hyUSD, and
/// sHYUSD. Useful for simulations of mint and redemption.
// NOTE: Wallet holding reference token balances, used for simulating transactions.
pub const REFERENCE_WALLET: Pubkey;

// NOTE: Wallet holding reference token balances, used for simulating transactions.
pub const REFERENCE_WALLET: Pubkey;


---

# Crate: hylo_core (lib)

# crate
<!-- file: hylo-core/src/lib.rs -->

## Re-exports

// NOTE: Re-export of hylo_idl as the idl module in hylo-core.
pub use hylo_idl as idl;


---

# crate::asset_swap_config
<!-- file: hylo-core/src/asset_swap_config.rs -->

## Types

pub struct AssetSwapConfig {
    pub fee: UFix64 < N4 >,
}


## Impl AssetSwapConfig

impl AssetSwapConfig {
    pub fn new(serialized_fee : UFixValue64) -> Result < AssetSwapConfig >;
    pub fn apply_fee< Exp >(& self, amount : UFix64 < Exp >) -> Result < FeeExtract < Exp > >;
    pub fn validate_fee(fee : UFix64 < N4 >) -> Result < () >;
}


## Constants

/// 100 bps (1%)
const MAX_FEE: UFix64 < N4 >;


---

# crate::conversion
<!-- file: hylo-core/src/conversion.rs -->

## Types

/// Provides conversions between an LST and protocol tokens.
// NOTE: Provides bidirectional price conversion between LST collateral and protocol tokens using oracle prices.
pub struct Conversion {
    pub usd_sol_price: PriceRange < N9 >,
    pub lst_sol_price: UFix64 < N9 >,
}

/// Conversions between the protocol's tokens.
// NOTE: Converts between stablecoin and levercoin using their respective NAVs.
pub struct SwapConversion {
    pub stablecoin_nav: UFix64 < N9 >,
    pub levercoin_nav: PriceRange < N9 >,
}

/// Conversions between an exogenous collateral and protocol tokens.
// NOTE: Provides bidirectional price conversion between exogenous collateral and protocol tokens.
pub struct ExoConversion {
    collateral_usd_price: PriceRange < N9 >,
}

/// Directional conversion between USDC and stablecoin amounts.
pub struct UsdcStablecoinConversion {
    pub usdc_usd_price: PriceRange < N9 >,
}

/// Conversions between exogenous collateral and USDC via oracle prices.
pub struct ExoRebalanceConversion {
    collateral_usd_price: UFix64 < N9 >,
    usdc_usd_price: PriceRange < N9 >,
}

/// Conversions between LST and USDC via SOL for rebalancing.
pub struct LstRebalanceConversion {
    lst_sol: UFix64 < N9 >,
    sol_usd: UFix64 < N9 >,
    usdc_usd: PriceRange < N9 >,
}


## Impl Conversion

// NOTE: Provides bidirectional price conversion between LST collateral and protocol tokens using oracle prices.
impl Conversion {
    pub fn new(usd_sol_price : PriceRange < N9 >, lst_sol_price : UFix64 < N9 >) -> Self;
    pub fn spot(usd_sol_price : UFix64 < N9 >, lst_sol_price : UFix64 < N9 >) -> Conversion;
    pub fn lst_to_token(& self, amount_lst : UFix64 < N9 >, token_nav : UFix64 < N9 >) -> Result < UFix64 < N6 > >;
    pub fn token_to_lst(& self, amount_token : UFix64 < N6 >, token_nav : UFix64 < N9 >) -> Result < UFix64 < N9 > >;
}


## Impl SwapConversion

// NOTE: Converts between stablecoin and levercoin using their respective NAVs.
impl SwapConversion {
    pub fn new(stablecoin_nav : UFix64 < N9 >, levercoin_nav : PriceRange < N9 >) -> Self;
    pub fn stable_to_lever(& self, amount_stable : UFix64 < N6 >) -> Result < UFix64 < N6 > >;
    pub fn lever_to_stable(& self, amount_lever : UFix64 < N6 >) -> Result < UFix64 < N6 > >;
}


## Impl ExoConversion

// NOTE: Provides bidirectional price conversion between exogenous collateral and protocol tokens.
impl ExoConversion {
    pub fn new(collateral_usd_price : PriceRange < N9 >) -> ExoConversion;
    pub fn spot(collateral_usd_price : UFix64 < N9 >) -> ExoConversion;
    pub fn exo_to_token(& self, amount : UFix64 < N9 >, token_nav : UFix64 < N9 >) -> Result < UFix64 < N6 > >;
    pub fn token_to_exo(& self, amount : UFix64 < N6 >, token_nav : UFix64 < N9 >) -> Result < UFix64 < N9 > >;
}


## Impl UsdcStablecoinConversion

impl UsdcStablecoinConversion {
    pub fn deposit_to_stablecoin(& self, usdc_amount : UFix64 < N9 >) -> Result < UFix64 < N6 > >;
    pub fn stablecoin_to_withdrawal(& self, stablecoin_amount : UFix64 < N6 >) -> Result < UFix64 < N9 > >;
    pub fn withdrawal_to_stablecoin(& self, usdc_amount : UFix64 < N9 >) -> Result < UFix64 < N6 > >;
}


## Impl ExoRebalanceConversion

impl ExoRebalanceConversion {
    pub fn new(collateral_usd_price : UFix64 < N9 >, usdc_usd_price : PriceRange < N9 >) -> ExoRebalanceConversion;
    pub fn collateral_to_usdc(& self, collateral_amount : UFix64 < N9 >) -> Result < UFix64 < N9 > >;
    pub fn usdc_to_collateral(& self, usdc_amount : UFix64 < N9 >) -> Result < UFix64 < N9 > >;
}


## Impl LstRebalanceConversion

impl LstRebalanceConversion {
    pub fn new(lst_sol : UFix64 < N9 >, sol_usd : UFix64 < N9 >, usdc_usd : PriceRange < N9 >) -> LstRebalanceConversion;
    pub fn lst_to_usdc(& self, lst_amount : UFix64 < N9 >) -> Result < UFix64 < N9 > >;
    pub fn usdc_to_lst(& self, usdc_amount : UFix64 < N9 >) -> Result < UFix64 < N9 > >;
}


---

# crate::error
<!-- file: hylo-core/src/error.rs -->

## Types

// NOTE: Comprehensive error enum covering all protocol math, oracle validation, fee, and arithmetic failures.
pub enum CoreError {
    TotalSolCacheDecrement,
    TotalSolCacheIncrement,
    TotalSolCacheOverflow,
    TotalSolCacheUnderflow,
    TotalSolCacheOutdated,
    LstSolPriceDelta,
    LstSolPriceEpochOrder,
    LstSolPriceOutdated,
    LstSolPriceConversion,
    SolLstPriceConversion,
    LstLstPriceConversion,
    PythOracleConfidence,
    PythOracleExponent,
    PythOracleNegativePrice,
    PythOracleNegativeTime,
    PythOracleOutdated,
    PythOraclePriceRange,
    PythOracleSlotInvalid,
    PythOracleVerificationLevel,
    CollateralRatio,
    MaxMintable,
    MaxSwappable,
    StablecoinNav,
    TargetCollateralRatioTooLow,
    TotalValueLocked,
    SlippageArithmetic,
    SlippageExceeded,
    StabilityValidation,
    LeverToStable,
    StableToLever,
    LstToToken,
    TokenToLst,
    FeeExtraction,
    NoValidLevercoinMintFee,
    NoValidLevercoinRedeemFee,
    NoValidStablecoinMintFee,
    NoValidSwapFee,
    InvalidFees,
    LevercoinNav,
    DestinationCollateral,
    DestinationStablecoin,
    NoNextStabilityThreshold,
    RequestedStablecoinOverMaxMintable,
    LpTokenNav,
    LpTokenOut,
    TokenWithdraw,
    YieldHarvestConfigValidation,
    YieldHarvestAllocation,
    MintOverflow,
    BurnUnderflow,
    InterpInsufficientPoints,
    InterpPointsNotMonotonic,
    InterpOutOfDomain,
    InterpArithmetic,
    CollateralRatioConversion,
    InterpFeeConversion,
    FundingRateValidation,
    FundingRateApply,
    ExoToToken,
    ExoFromToken,
    ExoAmountNormalization,
    ExoCollateralToUsdc,
    ExoUsdcToCollateral,
    LstToUsdc,
    UsdcToLst,
    RebalanceCurveConfigValidation,
    RebalancePriceConstruction,
    RebalancePriceConversion,
    RebalanceOutOfDomain,
    RebalanceAmountExceeded,
    RebalanceSellSideLiquidity,
    RebalanceBuySideTarget,
    StakePoolDivByZero,
}


---

# crate::exchange_context
<!-- file: hylo-core/src/exchange_context/mod.rs -->

## Traits

/// Shared interface for exchange context implementations.
// NOTE: Trait providing shared exchange state: collateral ratio, NAVs, fees, and mint/swap capacity.
pub trait ExchangeContext {
    fn total_collateral(& self) -> UFix64 < N9 >;
    fn collateral_usd_price(& self) -> PriceRange < N9 >;
    fn collateral_oracle_price(& self) -> OraclePrice;
    fn sell_curve_config(& self) -> & RebalanceCurveConfig;
    fn buy_curve_config(& self) -> & RebalanceCurveConfig;
    fn rebalance_sell_curve(& self) -> Result < SellPriceCurve >;
    fn rebalance_buy_curve(& self) -> Result < BuyPriceCurve >;
    fn rebalance_sell_active(& self) -> bool;
    fn rebalance_buy_active(& self) -> bool;
    fn rebalance_sell_liquidity(& self) -> Result < UFix64 < N9 > >;
    fn rebalance_buy_target(& self) -> Result < UFix64 < N9 > >;
    fn virtual_stablecoin_supply(& self) -> Result < UFix64 < N6 > >;
    fn levercoin_supply(& self) -> Result < UFix64 < N6 > >;
    fn stability_controller(& self) -> & StabilityController;
    fn stability_mode(& self) -> StabilityMode;
    fn collateral_ratio(& self) -> UFix64 < N9 >;
    fn levercoin_fees(& self) -> & LevercoinFees;
    fn total_value_locked(& self) -> Result < UFix64 < N9 > >;
    fn stablecoin_nav(& self) -> Result < UFix64 < N9 > >;
    fn levercoin_mint_nav(& self) -> Result < UFix64 < N9 > >;
    fn levercoin_redeem_nav(& self) -> Result < UFix64 < N9 > >;
    fn projected_stability_mode(& self, new_total : UFix64 < N9 >, new_stablecoin : UFix64 < N6 >) -> Result < StabilityMode >;
    fn select_stability_mode_for_fees(& self, projected : StabilityMode) -> StabilityMode;
    fn swap_conversion(& self) -> Result < SwapConversion >;
    fn max_mintable_stablecoin(& self) -> Result < UFix64 < N6 > >;
    fn max_swappable_stablecoin(& self) -> Result < UFix64 < N6 > >;
    fn validate_stablecoin_amount(& self, requested : UFix64 < N6 >) -> Result < UFix64 < N6 > >;
    fn validate_stablecoin_swap_amount(& self, requested : UFix64 < N6 >) -> Result < UFix64 < N6 > >;
    fn levercoin_to_stablecoin_fee(& self, amount_stablecoin : UFix64 < N6 >) -> Result < FeeExtract < N6 > >;
    fn stablecoin_to_levercoin_fee(& self, amount_stablecoin : UFix64 < N6 >) -> Result < FeeExtract < N6 > >;
}


## Functions

/// Ensures ST1 is strictly above ST2 (derived from the redeem fee curve).
/// 
/// # Errors
/// * Thresholds fail validation
pub fn validate_stability_thresholds(stability_threshold_1 : UFix64 < N2 >, stability_threshold_2 : UFix64 < N2 >) -> Result < () >;


## Re-exports

// NOTE: Re-export of ExoExchangeContext from the exchange_context module.
pub use self :: exo :: ExoExchangeContext;

// NOTE: Re-export of LstExchangeContext from the exchange_context module.
pub use self :: lst :: LstExchangeContext;


---

# crate::exchange_context::exo
<!-- file: hylo-core/src/exchange_context/exo.rs -->

## Types

/// Exchange context for exogenous collateral pairs.
// NOTE: Exchange context for exogenous (non-SOL) collateral pairs with interpolated fee curves and virtual stablecoin tracking.
pub struct ExoExchangeContext< C > {
    pub clock: C,
    pub total_collateral: UFix64 < N9 >,
    pub collateral_oracle: OraclePrice,
    pub collateral_usd_price: PriceRange < N9 >,
    pub virtual_stablecoin: VirtualStablecoin,
    levercoin_supply: Option < UFix64 < N6 > >,
    collateral_ratio: UFix64 < N9 >,
    stability_mode: StabilityMode,
    pub stability_controller: StabilityController,
    levercoin_fees: LevercoinFees,
    stablecoin_mint_fees: InterpolatedMintFees,
    stablecoin_redeem_fees: InterpolatedRedeemFees,
    sell_curve_config: RebalanceCurveConfig,
    buy_curve_config: RebalanceCurveConfig,
}


## Impl ExchangeContext for ExoExchangeContext < C >

// NOTE: Implements ExchangeContext for exogenous collateral using virtual stablecoin supply and interpolated fees.
impl < C : SolanaClock >ExchangeContext for ExoExchangeContext < C > {
    fn total_collateral(& self) -> UFix64 < N9 >;
    fn collateral_usd_price(& self) -> PriceRange < N9 >;
    fn collateral_oracle_price(& self) -> OraclePrice;
    fn sell_curve_config(& self) -> & RebalanceCurveConfig;
    fn buy_curve_config(& self) -> & RebalanceCurveConfig;
    fn virtual_stablecoin_supply(& self) -> Result < UFix64 < N6 > >;
    fn levercoin_supply(& self) -> Result < UFix64 < N6 > >;
    fn stability_controller(& self) -> & StabilityController;
    fn stability_mode(& self) -> StabilityMode;
    fn collateral_ratio(& self) -> UFix64 < N9 >;
    fn levercoin_fees(& self) -> & LevercoinFees;
}


## Impl ExoExchangeContext < C >

// NOTE: Inherent methods for loading exo context from on-chain state and computing fees for each operation type.
impl < C : SolanaClock >ExoExchangeContext < C > {
    pub fn load(clock : C, total_collateral : UFix64 < N9 >, stability_threshold_1 : UFix64 < N2 >, oracle_config : OracleConfig, levercoin_fees : LevercoinFees, collateral_usd_pyth_feed : & PriceUpdateV2, virtual_stablecoin : VirtualStablecoin, levercoin_mint : Option < & Mint >, sell_curve_config : RebalanceCurveConfig, buy_curve_config : RebalanceCurveConfig) -> Result < ExoExchangeContext < C > >;
    pub fn stablecoin_mint_fee(& self, collateral_amount : UFix64 < N9 >) -> Result < FeeExtract < N9 > >;
    pub fn stablecoin_redeem_fee(& self, collateral_amount : UFix64 < N9 >) -> Result < FeeExtract < N9 > >;
    pub fn levercoin_mint_fee(& self, collateral_amount : UFix64 < N9 >) -> Result < FeeExtract < N9 > >;
    pub fn levercoin_redeem_fee(& self, collateral_amount : UFix64 < N9 >) -> Result < FeeExtract < N9 > >;
    pub fn exo_conversion(& self) -> ExoConversion;
    pub fn rebalance_sell_conversion(& self, usdc_usd_price : PriceRange < N9 >, usdc_amount : UFix64 < N9 >) -> Result < ExoRebalanceConversion >;
    pub fn rebalance_buy_conversion(& self, usdc_usd_price : PriceRange < N9 >, collateral_amount : UFix64 < N9 >) -> Result < ExoRebalanceConversion >;
}


---

# crate::exchange_context::lst
<!-- file: hylo-core/src/exchange_context/lst.rs -->

## Types

/// Exchange context for SOL/LST collateral pairs.
// NOTE: Exchange context for SOL/LST collateral pairs using real mint supplies and table-based fees.
pub struct LstExchangeContext< C > {
    pub clock: C,
    pub total_sol: UFix64 < N9 >,
    pub sol_usd_oracle: OraclePrice,
    pub sol_usd_price: PriceRange < N9 >,
    virtual_stablecoin: VirtualStablecoin,
    levercoin_supply: Option < UFix64 < N6 > >,
    collateral_ratio: UFix64 < N9 >,
    pub stability_controller: StabilityController,
    stability_mode: StabilityMode,
    stablecoin_mint_fees: InterpolatedMintFees,
    stablecoin_redeem_fees: InterpolatedRedeemFees,
    levercoin_fees: LevercoinFees,
    sell_curve_config: RebalanceCurveConfig,
    buy_curve_config: RebalanceCurveConfig,
}


## Impl ExchangeContext for LstExchangeContext < C >

// NOTE: Implements ExchangeContext for SOL/LST collateral using TotalSolCache and SOL/USD oracle.
impl < C : SolanaClock >ExchangeContext for LstExchangeContext < C > {
    fn total_collateral(& self) -> UFix64 < N9 >;
    fn collateral_usd_price(& self) -> PriceRange < N9 >;
    fn collateral_oracle_price(& self) -> OraclePrice;
    fn sell_curve_config(& self) -> & RebalanceCurveConfig;
    fn buy_curve_config(& self) -> & RebalanceCurveConfig;
    fn virtual_stablecoin_supply(& self) -> Result < UFix64 < N6 > >;
    fn levercoin_supply(& self) -> Result < UFix64 < N6 > >;
    fn stability_controller(& self) -> & StabilityController;
    fn stability_mode(& self) -> StabilityMode;
    fn collateral_ratio(& self) -> UFix64 < N9 >;
    fn levercoin_fees(& self) -> & LevercoinFees;
}


## Impl LstExchangeContext < C >

// NOTE: Inherent methods for loading LST context, computing fees, token conversions, and pool caps.
impl < C : SolanaClock >LstExchangeContext < C > {
    pub fn load(clock : C, total_sol_cache : & TotalSolCache, stability_threshold_1 : UFix64 < N2 >, oracle_config : OracleConfig, levercoin_fees : LevercoinFees, sol_usd_pyth_feed : & PriceUpdateV2, virtual_stablecoin : VirtualStablecoin, levercoin_mint : Option < & Mint >, sell_curve_config : RebalanceCurveConfig, buy_curve_config : RebalanceCurveConfig) -> Result < LstExchangeContext < C > >;
    pub fn stablecoin_mint_fee(& self, lst_sol_price : & LstSolPrice, amount_lst : UFix64 < N9 >) -> Result < FeeExtract < N9 > >;
    pub fn stablecoin_redeem_fee(& self, lst_sol_price : & LstSolPrice, amount_lst : UFix64 < N9 >) -> Result < FeeExtract < N9 > >;
    pub fn levercoin_mint_fee(& self, lst_sol_price : & LstSolPrice, amount_lst : UFix64 < N9 >) -> Result < FeeExtract < N9 > >;
    pub fn levercoin_redeem_fee(& self, lst_sol_price : & LstSolPrice, amount_lst : UFix64 < N9 >) -> Result < FeeExtract < N9 > >;
    pub fn token_conversion(& self, lst_sol_price : & LstSolPrice) -> Result < Conversion >;
    pub fn sol_to_stablecoin(& self, amount_sol : UFix64 < N9 >) -> Result < UFix64 < N6 > >;
    pub fn sol_to_levercoin(& self, amount_sol : UFix64 < N9 >) -> Result < UFix64 < N6 > >;
    pub fn rebalance_sell_conversion(& self, lst_sol_price : & LstSolPrice, usdc_usd_price : PriceRange < N9 >, usdc_amount : UFix64 < N9 >) -> Result < LstRebalanceConversion >;
    pub fn rebalance_buy_conversion(& self, lst_sol_price : & LstSolPrice, usdc_usd_price : PriceRange < N9 >, lst_amount : UFix64 < N9 >) -> Result < LstRebalanceConversion >;
    pub fn max_swappable_stablecoin_to_next_threshold(& self) -> Result < UFix64 < N6 > >;
}


---

# crate::exchange_math
<!-- file: hylo-core/src/exchange_math.rs -->

## Functions

/// Computes the current collateral ratio (CR) of the protocol.
///   `CR = total_sol_usd / stablecoin_cap`
/// 
/// NB: If stablecoin supply is zero, returns `u64::MAX` to simulate infinity.
// NOTE: Computes CR = total_collateral_usd / stablecoin_supply, returning u64::MAX when supply is zero.
pub fn collateral_ratio(total_collateral : UFix64 < N9 >, usd_collateral_price : UFix64 < N9 >, amount_stablecoin : UFix64 < N6 >) -> Result < UFix64 < N9 > >;

/// Multiples total SOL by the given spot price to get TVL.
// NOTE: Multiplies total collateral by USD price to compute protocol TVL.
pub fn total_value_locked(total_collateral : UFix64 < N9 >, usd_collateral_price : UFix64 < N9 >) -> Result < UFix64 < N9 > >;

/// Given the next collateral ratio threshold below the current, determines the
/// amount of stablecoin that can safely be minted.
/// 
/// Finds `max_stablecoin` assuming stablecoin NAV is $1.
///   `max_stablecoin = (tvl - target_cr * cur_stablecoin) / (target_cr - 1)`
// NOTE: Computes maximum stablecoin mintable before hitting the next collateral ratio threshold.
pub fn max_mintable_stablecoin(target_collateral_ratio : UFix64 < N2 >, total_collateral : UFix64 < N9 >, usd_collateral_price : UFix64 < N9 >, stablecoin_supply : UFix64 < N6 >) -> Result < UFix64 < N6 > >;

/// Without changing TVL, computes how much stablecoin can be swapped from
/// levercoin.
/// 
/// ```txt
///                   total_value_locked
/// max_swappable = -----------------------  - stablecoin_supply
///                 target_collateral_ratio
/// ```
// NOTE: Computes maximum stablecoin obtainable via swap without breaching the target collateral ratio.
pub fn max_swappable_stablecoin(target_collateral_ratio : UFix64 < N2 >, total_value_locked : UFix64 < N9 >, stablecoin_supply : UFix64 < N6 >) -> Result < UFix64 < N6 > >;

/// Computes upper bound of levercoin NAV for minting.
/// 
/// If the current supply of the levercoin is zero, the price is $1.
/// 
/// Otherwise its NAV is computed as:
///   `free_collateral = (n_collateral * p_collateral) - (n_stable * p_stable)`
///   `new_nav = free_collateral / n_lever`
// NOTE: Computes upper-bound levercoin NAV for minting: free_collateral / levercoin_supply.
pub fn next_levercoin_mint_nav(total_collateral : UFix64 < N9 >, usd_collateral_price : PriceRange < N9 >, stablecoin_supply : UFix64 < N6 >, stablecoin_nav : UFix64 < N9 >, levercoin_supply : UFix64 < N6 >) -> Option < UFix64 < N9 > >;

/// Computes lower bound of levercoin NAV for redemption.
// NOTE: Computes lower-bound levercoin NAV for redemption.
pub fn next_levercoin_redeem_nav(total_collateral : UFix64 < N9 >, usd_collateral_price : PriceRange < N9 >, stablecoin_supply : UFix64 < N6 >, stablecoin_nav : UFix64 < N9 >, levercoin_supply : UFix64 < N6 >) -> Option < UFix64 < N9 > >;

/// Computes stablecoin NAV during a depeg scenario.
/// In all other modes, the price of the stablecoin is fixed to $1.
///   `NAV = total_sol * sol_usd_price / supply`
// NOTE: Computes stablecoin NAV during depeg mode as total_collateral_usd / supply.
pub fn depeg_stablecoin_nav(total_collateral : UFix64 < N9 >, usd_collateral_price : UFix64 < N9 >, stablecoin_supply : UFix64 < N6 >) -> Result < UFix64 < N9 > >;


---

# crate::fee_controller
<!-- file: hylo-core/src/fee_controller.rs -->

## Types

/// Represents the spread of fees between mint and redeem for protocol tokens.
/// All fees must be in basis points to represent a fractional percentage
/// directly applicable to a token amount e.g. `0.XXXX` or `bips x 10^-4`.
// NOTE: A mint/redeem fee pair stored as raw UFixValue64 values in basis points.
pub struct FeePair {
    pub mint: UFixValue64,
    pub redeem: UFixValue64,
}

/// Combines fee multiplication for a token amount with the remaining token
/// amount by subtraction.
// NOTE: Result of fee extraction: the extracted fee amount and the remaining amount after deduction.
pub struct FeeExtract< Exp > {
    pub fees_extracted: UFix64 < Exp >,
    pub amount_remaining: UFix64 < Exp >,
}

/// **Deprecated** — retained only for `Hylo` account deserialization.
// NOTE: Fee table for stablecoin operations with two tiers (normal, mode_1).
pub struct StablecoinFees {
    pub normal: FeePair,
    pub mode_1: FeePair,
}

// NOTE: Fee table for levercoin operations with three tiers (normal, mode_1, mode_2) plus swap fees.
pub struct LevercoinFees {
    pub normal: FeePair,
    pub mode_1: FeePair,
    pub mode_2: FeePair,
}


## Traits

/// Fee configuration table reacts to different stability modes.
// NOTE: Trait that selects mint/redeem fee rates based on the current stability mode.
pub trait FeeController: Sized {
    fn mint_fee(& self, mode : StabilityMode) -> Result < UFix64 < N4 > >;
    fn redeem_fee(& self, mode : StabilityMode) -> Result < UFix64 < N4 > >;
    fn validate(self) -> Result < Self >;
}


## Impl FeePair

// NOTE: A mint/redeem fee pair stored as raw UFixValue64 values in basis points.
impl FeePair {
    pub fn new(mint : UFixValue64, redeem : UFixValue64) -> FeePair;
    pub fn mint(& self) -> Result < UFix64 < N4 > >;
    pub fn redeem(& self) -> Result < UFix64 < N4 > >;
    pub fn validate(& self) -> Result < () >;
}


## Impl FeeExtract < Exp >

// NOTE: Constructor that computes fee extraction from a basis-point fee and input amount.
impl < Exp >FeeExtract < Exp > {
    pub fn new< FeeExp >(fee : UFix64 < FeeExp >, amount_in : UFix64 < Exp >) -> Result < FeeExtract < Exp > >;
}


## Impl StablecoinFees

// NOTE: Fee table for stablecoin operations with two tiers (normal, mode_1).
impl StablecoinFees {
    pub fn new(normal : FeePair, mode_1 : FeePair) -> StablecoinFees;
}


## Impl FeeController for LevercoinFees

// NOTE: Selects levercoin mint/redeem fees from normal, mode_1, or mode_2 fee tables.
impl FeeController for LevercoinFees {
    fn mint_fee(& self, mode : StabilityMode) -> Result < UFix64 < N4 > >;
    fn redeem_fee(& self, mode : StabilityMode) -> Result < UFix64 < N4 > >;
    fn validate(self) -> Result < LevercoinFees >;
}


## Impl LevercoinFees

// NOTE: Fee table for levercoin operations with three tiers (normal, mode_1, mode_2) plus swap fees.
impl LevercoinFees {
    pub fn new(normal : FeePair, mode_1 : FeePair, mode_2 : FeePair) -> LevercoinFees;
    pub fn swap_to_stablecoin_fee(& self, mode : StabilityMode) -> Result < UFix64 < N4 > >;
    pub fn swap_from_stablecoin_fee(& self, mode : StabilityMode) -> Result < UFix64 < N4 > >;
}


## Constants

/// 1000 bps (10%)
const MAX_FEE: UFix64 < N4 >;


---

# crate::fee_curves
<!-- file: hylo-core/src/fee_curves.rs -->

## Functions

/// Loads the mint fee curve into an interpolator.
/// 
/// # Errors
/// * Curve validation
// NOTE: Returns the piecewise-linear interpolator for collateral-ratio-dependent mint fees.
pub fn mint_fee_curve() -> Result < FixInterp < 21 , N5 > >;

/// Loads the redeem fee curve into an interpolator.
/// 
/// # Errors
/// * Curve validation
// NOTE: Returns the piecewise-linear interpolator for collateral-ratio-dependent redeem fees.
pub fn redeem_fee_curve() -> Result < FixInterp < 20 , N5 > >;


## Macros

// NOTE: Macro that defines a fee curve as an array of fixed-point coordinate pairs.
macro_rules! generate_curve { ... }


---

# crate::funding_rate
<!-- file: hylo-core/src/funding_rate.rs -->

## Types

/// Per-epoch funding rate for exogenous collateral without native yield.
// NOTE: Per-epoch funding rate for exogenous collateral, capped at ~10% annualized.
pub struct FundingRateConfig {
    pub rate: UFixValue64,
    pub fee: UFixValue64,
}


## Impl FundingRateConfig

// NOTE: Per-epoch funding rate for exogenous collateral, capped at ~10% annualized.
impl FundingRateConfig {
    pub fn new(rate : UFixValue64, fee : UFixValue64) -> FundingRateConfig;
    pub fn rate(& self) -> Result < UFix64 < N9 > >;
    pub fn fee(& self) -> Result < UFix64 < N4 > >;
    pub fn apply_funding_rate(& self, amount : UFix64 < N9 >) -> Result < UFix64 < N9 > >;
    pub fn apply_fee(& self, amount : UFix64 < N6 >) -> Result < FeeExtract < N6 > >;
    pub fn validate(& self) -> Result < FundingRateConfig >;
}


## Constants

/// Maximum per-epoch rate (~10% annualized at 182 epochs/year)
// NOTE: Maximum per-epoch funding rate (~10% annualized at 182 epochs/year).
const MAX_RATE: UFix64 < N9 >;

/// Maximum fee exacted against funding rate
const MAX_FEE: UFix64 < N4 >;


---

# crate::idl_type_bridge
<!-- file: hylo-core/src/idl_type_bridge.rs -->

## Impl From < hylo_idl :: exchange :: types :: LstSolPrice > for LstSolPrice

// NOTE: Converts IDL LstSolPrice into the core domain type.
impl From < hylo_idl :: exchange :: types :: LstSolPrice > for LstSolPrice {
    fn from(idl : hylo_idl :: exchange :: types :: LstSolPrice) -> Self;
}


## Impl From < hylo_idl :: exchange :: types :: StablecoinFees > for StablecoinFees

// NOTE: Converts IDL StablecoinFees into the core domain type.
impl From < hylo_idl :: exchange :: types :: StablecoinFees > for StablecoinFees {
    fn from(idl : hylo_idl :: exchange :: types :: StablecoinFees) -> StablecoinFees;
}


## Impl From < hylo_idl :: exchange :: types :: LevercoinFees > for LevercoinFees

// NOTE: Converts IDL LevercoinFees into the core domain type.
impl From < hylo_idl :: exchange :: types :: LevercoinFees > for LevercoinFees {
    fn from(idl : hylo_idl :: exchange :: types :: LevercoinFees) -> Self;
}


## Impl From < hylo_idl :: exchange :: types :: FeePair > for FeePair

// NOTE: Converts IDL FeePair into the core domain type.
impl From < hylo_idl :: exchange :: types :: FeePair > for FeePair {
    fn from(idl : hylo_idl :: exchange :: types :: FeePair) -> FeePair;
}


## Impl From < hylo_idl :: exchange :: types :: TotalSolCache > for TotalSolCache

// NOTE: Converts IDL TotalSolCache into the core domain type.
impl From < hylo_idl :: exchange :: types :: TotalSolCache > for TotalSolCache {
    fn from(idl : hylo_idl :: exchange :: types :: TotalSolCache) -> TotalSolCache;
}


## Impl From < hylo_idl :: exchange :: types :: YieldHarvestConfig > for YieldHarvestConfig

// NOTE: Converts IDL YieldHarvestConfig into the core domain type.
impl From < hylo_idl :: exchange :: types :: YieldHarvestConfig > for YieldHarvestConfig {
    fn from(idl : hylo_idl :: exchange :: types :: YieldHarvestConfig) -> Self;
}


## Impl From < hylo_idl :: exchange :: types :: HarvestCache > for HarvestCache

// NOTE: Converts IDL HarvestCache into the core domain type.
impl From < hylo_idl :: exchange :: types :: HarvestCache > for HarvestCache {
    fn from(idl : hylo_idl :: exchange :: types :: HarvestCache) -> Self;
}


## Impl From < hylo_idl :: exchange :: types :: VirtualStablecoin > for VirtualStablecoin

impl From < hylo_idl :: exchange :: types :: VirtualStablecoin > for VirtualStablecoin {
    fn from(idl : hylo_idl :: exchange :: types :: VirtualStablecoin) -> VirtualStablecoin;
}


## Impl From < hylo_idl :: exchange :: types :: FundingRateConfig > for FundingRateConfig

impl From < hylo_idl :: exchange :: types :: FundingRateConfig > for FundingRateConfig {
    fn from(idl : hylo_idl :: exchange :: types :: FundingRateConfig) -> FundingRateConfig;
}


## Impl From < hylo_idl :: exchange :: types :: RebalanceCurveConfig > for RebalanceCurveConfig

impl From < hylo_idl :: exchange :: types :: RebalanceCurveConfig > for RebalanceCurveConfig {
    fn from(idl : hylo_idl :: exchange :: types :: RebalanceCurveConfig) -> RebalanceCurveConfig;
}


## Impl From < SlippageConfig > for hylo_idl :: exchange :: types :: SlippageConfig

// NOTE: Converts core SlippageConfig into the IDL representation for instruction building.
impl From < SlippageConfig > for hylo_idl :: exchange :: types :: SlippageConfig {
    fn from(val : SlippageConfig) -> Self;
}


## Impl From < SlippageConfig > for hylo_idl :: router :: types :: SlippageConfig

impl From < SlippageConfig > for hylo_idl :: router :: types :: SlippageConfig {
    fn from(val : SlippageConfig) -> Self;
}


## Impl From < FeePair > for hylo_idl :: exchange :: types :: FeePair

impl From < FeePair > for hylo_idl :: exchange :: types :: FeePair {
    fn from(val : FeePair) -> Self;
}


## Impl From < StablecoinFees > for hylo_idl :: exchange :: types :: StablecoinFees

impl From < StablecoinFees > for hylo_idl :: exchange :: types :: StablecoinFees {
    fn from(val : StablecoinFees) -> Self;
}


## Impl From < LevercoinFees > for hylo_idl :: exchange :: types :: LevercoinFees

impl From < LevercoinFees > for hylo_idl :: exchange :: types :: LevercoinFees {
    fn from(val : LevercoinFees) -> Self;
}


## Impl From < YieldHarvestConfig > for hylo_idl :: exchange :: types :: YieldHarvestConfig

impl From < YieldHarvestConfig > for hylo_idl :: exchange :: types :: YieldHarvestConfig {
    fn from(val : YieldHarvestConfig) -> Self;
}


## Impl From < RebalanceCurveConfig > for hylo_idl :: exchange :: types :: RebalanceCurveConfig

impl From < RebalanceCurveConfig > for hylo_idl :: exchange :: types :: RebalanceCurveConfig {
    fn from(val : RebalanceCurveConfig) -> Self;
}


## Impl From < FundingRateConfig > for hylo_idl :: exchange :: types :: FundingRateConfig

impl From < FundingRateConfig > for hylo_idl :: exchange :: types :: FundingRateConfig {
    fn from(val : FundingRateConfig) -> Self;
}


---

# crate::interp
<!-- file: hylo-core/src/interp.rs -->

## Types

/// Fixed-point Cartesian coordinate.
// NOTE: Fixed-point Cartesian coordinate with signed x and y components.
pub struct Point< Exp : Integer > {
    pub x: IFix64 < Exp >,
    pub y: IFix64 < Exp >,
}

/// Line segment between two points for linear interpolation.
// NOTE: A pair of Points defining a line segment for linear interpolation.
pub struct LineSegment< 'a , Exp : Integer >(& 'a Point < Exp >, & 'a Point < Exp >);

/// Piecewise linear interpolation over a fixed-size point array.
// NOTE: Piecewise linear interpolator over a compile-time-sized array of fixed-point points.
pub struct FixInterp< const RES : usize , Exp : Integer > {
    points: [Point < Exp > ; RES],
}


## Impl Point < Exp >

// NOTE: Constructor creating a Point from integer x and y values.
impl < Exp : Integer >Point < Exp > {
    pub fn from_ints(x : i64, y : i64) -> Point < Exp >;
}


## Impl LineSegment < '_ , Exp >

// NOTE: Linear interpolation (lerp) between two fixed-point endpoints.
impl < Exp : Integer >LineSegment < '_ , Exp > {
    pub fn lerp(& self, x : IFix64 < Exp >) -> Option < IFix64 < Exp > >;
}


## Impl FixInterp < RES , Exp >

// NOTE: Methods for constructing from points, querying domain/range bounds, and interpolating.
impl < const RES : usize , Exp : Integer >FixInterp < RES , Exp > {
    pub fn from_points(points : [Point < Exp > ; RES]) -> Result < Self >;
    pub fn from_points_unchecked(points : [Point < Exp > ; RES]) -> Self;
    pub fn x_min(& self) -> IFix64 < Exp >;
    pub fn x_max(& self) -> IFix64 < Exp >;
    pub fn y_min(& self) -> IFix64 < Exp >;
    pub fn y_max(& self) -> IFix64 < Exp >;
    pub fn interpolate(& self, x : IFix64 < Exp >) -> Result < IFix64 < Exp > >;
}


---

# crate::interpolated_fees
<!-- file: hylo-core/src/interpolated_fees.rs -->

## Types

// NOTE: Wrapper around a FixInterp curve for computing collateral-ratio-dependent mint fees.
pub struct InterpolatedMintFees {
    curve: FixInterp < 21 , N5 >,
}

// NOTE: Wrapper around a FixInterp curve for computing collateral-ratio-dependent redeem fees.
pub struct InterpolatedRedeemFees {
    curve: FixInterp < 20 , N5 >,
}


## Traits

/// Interpolated fee curve controller.
/// Implementors define boundary behavior via `fee_inner`.
// NOTE: Trait for fee controllers that use a piecewise-linear curve indexed by collateral ratio.
pub trait InterpolatedFeeController< const RES : usize > {
    fn curve(& self) -> & FixInterp < RES , N5 >;
    fn fee_inner(& self, cr : IFix64 < N5 >) -> Result < IFix64 < N5 > >;
    fn apply_fee< InExp >(& self, ucr : UFix64 < N9 >, amount_in : UFix64 < InExp >) -> Result < FeeExtract < InExp > >;
    fn cr_floor(& self) -> Result < UFix64 < N2 > >;
}


## Functions

/// Downconvert CR from `N9` unsigned to `N5` signed for curve lookup.
/// 
/// # Errors
/// * `CollateralRatioConversion` on `i64` overflow.
// NOTE: Converts unsigned N9 collateral ratio to signed N5 for fee curve lookup.
pub fn narrow_cr(cr : UFix64 < N9 >) -> Result < IFix64 < N5 > >;


## Impl InterpolatedMintFees

// NOTE: Wrapper around a FixInterp curve for computing collateral-ratio-dependent mint fees.
impl InterpolatedMintFees {
    pub fn new(curve : FixInterp < 21 , N5 >) -> InterpolatedMintFees;
}


## Impl InterpolatedFeeController < 21 > for InterpolatedMintFees

impl InterpolatedFeeController < 21 > for InterpolatedMintFees {
    fn curve(& self) -> & FixInterp < 21 , N5 >;
    fn fee_inner(& self, cr : IFix64 < N5 >) -> Result < IFix64 < N5 > >;
}


## Impl InterpolatedRedeemFees

// NOTE: Wrapper around a FixInterp curve for computing collateral-ratio-dependent redeem fees.
impl InterpolatedRedeemFees {
    pub fn new(curve : FixInterp < 20 , N5 >) -> InterpolatedRedeemFees;
}


## Impl InterpolatedFeeController < 20 > for InterpolatedRedeemFees

impl InterpolatedFeeController < 20 > for InterpolatedRedeemFees {
    fn curve(& self) -> & FixInterp < 20 , N5 >;
    fn fee_inner(& self, cr : IFix64 < N5 >) -> Result < IFix64 < N5 > >;
}


---

# crate::lst_sol_price
<!-- file: hylo-core/src/lst_sol_price.rs -->

## Types

/// Captures the true LST price in SOL for the current epoch.
// NOTE: Tracks an LST's SOL exchange rate for a specific epoch, with staleness and delta validation.
pub struct LstSolPrice {
    pub price: UFixValue64,
    pub epoch: u64,
}


## Impl LstSolPrice

// NOTE: Tracks an LST's SOL exchange rate for a specific epoch, with staleness and delta validation.
impl LstSolPrice {
    pub fn new(price : UFixValue64, epoch : u64) -> LstSolPrice;
    pub fn adjust_price(& self, fee : UFix64 < N5 >) -> Result < LstSolPrice >;
    pub fn checked_delta(& self, prev : & LstSolPrice) -> Result < UFix64 < N9 > >;
    pub fn get_epoch_price(& self, current_epoch : u64) -> Result < UFix64 < N9 > >;
    pub fn convert_lst_to_sol(& self, amount_lst : UFix64 < N9 >, current_epoch : u64) -> Result < UFix64 < N9 > >;
    pub fn convert_sol_to_lst(& self, amount_sol : UFix64 < N9 >, current_epoch : u64) -> Result < UFix64 < N9 > >;
    pub fn convert_lst_amount(& self, current_epoch : u64, amount_lst : UFix64 < N9 >, other : & LstSolPrice) -> Result < UFix64 < N9 > >;
}


---

# crate::pyth
<!-- file: hylo-core/src/pyth.rs -->

## Types

pub struct PythFeed {
    pub feed_id: FeedId,
    pub address: Pubkey,
}

// NOTE: Configuration for a Pyth oracle: staleness interval and confidence tolerance.
pub struct OracleConfig {
    pub interval_secs: u64,
    pub conf_tolerance: UFix64 < N9 >,
}

/// Spread of an asset price, with a lower and upper quote.
/// Use lower in minting, higher in redeeming.
// NOTE: Lower and upper price bounds derived from oracle price +/- confidence interval.
pub struct PriceRange< Exp : Integer > {
    pub lower: UFix64 < Exp >,
    pub upper: UFix64 < Exp >,
}

/// Validated oracle spot price and confidence interval.
pub struct OraclePrice {
    pub spot: UFix64 < N9 >,
    pub conf: UFix64 < N9 >,
}


## Functions

/// Checks the ratio of `conf / price` against given tolerance.
/// Guards against unusually large spreads in the oracle price.
// NOTE: Guards against unusually large oracle confidence/price ratios.
fn validate_conf(price : UFix64 < N9 >, conf : UFix64 < N9 >, tolerance : UFix64 < N9 >) -> Result < UFix64 < N9 > >;

/// Ensures the oracle's publish time is within the inclusive range:
///   `[clock_time - oracle_interval, clock_time]`
// NOTE: Ensures oracle publish time is within the allowed interval from current clock time.
fn validate_publish_time(publish_time : i64, oracle_interval : u64, clock_time : i64) -> Result < () >;

/// Number of Solana slots in configured oracle interval time.
// NOTE: Converts oracle interval seconds to Solana slot count.
fn slot_interval(oracle_interval_secs : u64) -> Option < u64 >;

/// Checks the posted slot of a price against the configured oracle interval.
// NOTE: Ensures the oracle's posted slot is within the configured staleness window.
fn validate_posted_slot(posted_slot : u64, oracle_interval_secs : u64, current_slot : u64) -> Result < () >;

/// Validates a Pyth price is positive and normalizes to `N9`.
/// 
/// # Errors
/// * Negative price or unsupported exponent
// NOTE: Validates Pyth price exponent matches target type and price is non-negative.
fn validate_price(price : i64, exp : i32) -> Result < UFix64 < N9 > >;

/// Normalizes a raw Pyth price to canonical `N9` precision.
/// Accepts Pyth exponents from `-2` through `-9`.
/// 
/// # Errors
/// * Unsupported exponent or conversion overflow
fn normalize_pyth_price(price : u64, exp : i32) -> Result < UFix64 < N9 > >;

/// Checks Pythnet verification level for the price update.
// NOTE: Checks that the Pyth price update meets the required Pythnet verification level.
fn validate_verification_level(level : VerificationLevel) -> Result < () >;

/// Fetches validated price and confidence from Pyth.
/// 
/// # Errors
/// * Validation
pub fn query_pyth_oracle< C : SolanaClock >(clock : & C, oracle : & PriceUpdateV2, OracleConfig { interval_secs , conf_tolerance , } : OracleConfig) -> Result < OraclePrice >;

/// Builds price range from Pyth oracle.
/// 
/// # Errors
/// * Validation
// NOTE: Fetches and validates a Pyth oracle price, returning a PriceRange with confidence bounds.
pub fn query_pyth_price< C : SolanaClock >(clock : & C, oracle : & PriceUpdateV2, config : OracleConfig) -> Result < PriceRange < N9 > >;


## Impl OracleConfig

// NOTE: Configuration for a Pyth oracle: staleness interval and confidence tolerance.
impl OracleConfig {
    pub fn new(interval_secs : u64, conf_tolerance : UFix64 < N9 >) -> OracleConfig;
}


## Impl PriceRange < Exp >

// NOTE: Constructors for PriceRange: from confidence spread, single price, or explicit bounds.
impl < Exp : Integer >PriceRange < Exp > {
    pub fn from_conf(price : UFix64 < Exp >, conf : UFix64 < Exp >) -> Result < PriceRange < Exp > >;
    pub fn one(price : UFix64 < Exp >) -> PriceRange < Exp >;
    pub fn new(lower : UFix64 < Exp >, upper : UFix64 < Exp >) -> PriceRange < Exp >;
}


## Impl OraclePrice

impl OraclePrice {
    pub fn price_range(& self) -> Result < PriceRange < N9 > >;
}


## Constants

// NOTE: Pyth feed ID for the SOL/USD price oracle.
pub const SOL_USD: PythFeed;

// NOTE: Pyth feed ID for the BTC/USD price oracle.
pub const BTC_USD: PythFeed;

pub const USDC_USD: PythFeed;


---

# crate::rebalance_math
<!-- file: hylo-core/src/rebalance_math.rs -->

## Functions

/// Max sellable collateral until exo pair CR rises to target.
/// 
/// ```text
///   target_cr * virtual_stablecoin - collateral_usd_price * total_collateral
///   ────────────────────────────────────────────────────────────────────────
///                  collateral_usd_price * (target_cr - 1)
/// ```
pub fn max_sellable_collateral(target_cr : UFix64 < N9 >, virtual_stablecoin : UFix64 < N6 >, collateral_usd_price : UFix64 < N9 >, total_collateral : UFix64 < N9 >) -> Option < UFix64 < N9 > >;

/// Max buyable collateral until exo pair CR falls to the target.
/// 
/// ```text
///   collateral_usd_price * total_collateral - target_cr * virtual_stablecoin
///   ────────────────────────────────────────────────────────────────────────
///                  collateral_usd_price * (target_cr - 1)
/// ```
pub fn max_buyable_collateral(target_cr : UFix64 < N9 >, virtual_stablecoin : UFix64 < N6 >, collateral_usd_price : UFix64 < N9 >, total_collateral : UFix64 < N9 >) -> Option < UFix64 < N9 > >;


---

# crate::rebalance_pricing
<!-- file: hylo-core/src/rebalance_pricing.rs -->

## Types

/// Confidence interval multipliers for rebalance price curve construction.
pub struct RebalanceCurveConfig {
    pub floor_mult: UFixValue64,
    pub ceil_mult: UFixValue64,
}

/// Sell side rebalance pricing curve.
/// Active when CR is low (below 1.35).
pub struct SellPriceCurve {
    curve: FixInterp < 2 , N9 >,
}

/// Buy-side rebalance pricing curve.
/// Active when CR is high (above 1.65).
pub struct BuyPriceCurve {
    curve: FixInterp < 2 , N9 >,
}


## Traits

/// Interpolated rebalance price controller.
/// Implementors define boundary behavior via [`price_inner`].
pub trait RebalancePriceController {
    fn curve(& self) -> & FixInterp < 2 , N9 >;
    fn is_active(& self, ucr : UFix64 < N9 >) -> bool;
    fn price_inner(& self, cr : IFix64 < N9 >) -> Result < IFix64 < N9 > >;
    fn price(& self, ucr : UFix64 < N9 >) -> Result < UFix64 < N9 > >;
    fn validate(self) -> Result < Self >;
}


## Functions

/// Convert unsigned CR to signed for curve lookup.
/// 
/// # Errors
/// * Conversion overflow
fn narrow_cr(cr : UFix64 < N9 >) -> Result < IFix64 < N9 > >;

/// Convert unsigned oracle price to signed for curve storage.
/// 
/// # Errors
/// * Conversion overflow
fn narrow_price(price : UFix64 < N9 >) -> Result < IFix64 < N9 > >;

/// Scales confidence interval by multiplier.
/// 
/// # Errors
/// * Arithmetic overflow
fn scale_ci(ci : UFix64 < N9 >, mult : UFix64 < N2 >) -> Result < UFix64 < N9 > >;


## Impl RebalanceCurveConfig

impl RebalanceCurveConfig {
    pub fn new(floor_mult : UFixValue64, ceil_mult : UFixValue64) -> RebalanceCurveConfig;
    pub fn floor_mult(& self) -> Result < UFix64 < N2 > >;
    pub fn ceil_mult(& self) -> Result < UFix64 < N2 > >;
    pub fn validate(self) -> Result < Self >;
}


## Impl SellPriceCurve

impl SellPriceCurve {
    pub fn new(OraclePrice { spot , conf } : OraclePrice, config : & RebalanceCurveConfig) -> Result < SellPriceCurve >;
}


## Impl RebalancePriceController for SellPriceCurve

impl RebalancePriceController for SellPriceCurve {
    fn curve(& self) -> & FixInterp < 2 , N9 >;
    fn is_active(& self, ucr : UFix64 < N9 >) -> bool;
    fn price_inner(& self, cr : IFix64 < N9 >) -> Result < IFix64 < N9 > >;
    fn validate(self) -> Result < SellPriceCurve >;
}


## Impl BuyPriceCurve

impl BuyPriceCurve {
    pub fn new(OraclePrice { spot , conf } : OraclePrice, config : & RebalanceCurveConfig) -> Result < BuyPriceCurve >;
}


## Impl RebalancePriceController for BuyPriceCurve

impl RebalancePriceController for BuyPriceCurve {
    fn curve(& self) -> & FixInterp < 2 , N9 >;
    fn is_active(& self, ucr : UFix64 < N9 >) -> bool;
    fn price_inner(& self, cr : IFix64 < N9 >) -> Result < IFix64 < N9 > >;
    fn validate(self) -> Result < BuyPriceCurve >;
}


## Constants

/// Sell curve upper boundary
pub const UCR_1_35: UFix64 < N9 >;

/// Buy curve lower boundary
pub const UCR_1_65: UFix64 < N9 >;

const CR_1_20: IFix64 < N9 >;

const CR_1_35: IFix64 < N9 >;

const CR_1_65: IFix64 < N9 >;

const CR_1_75: IFix64 < N9 >;


---

# crate::slippage_config
<!-- file: hylo-core/src/slippage_config.rs -->

## Types

/// Client specified slippage tolerance paired with expected token amount.
// NOTE: Client-specified slippage tolerance and expected output amount, validated on-chain before execution.
pub struct SlippageConfig {
    pub expected_token_out: UFixValue64,
    pub slippage_tolerance: UFixValue64,
}


## Impl SlippageConfig

// NOTE: Client-specified slippage tolerance and expected output amount, validated on-chain before execution.
impl SlippageConfig {
    pub fn new< Exp : Integer >(expected_token_out : UFix64 < Exp >, slippage_tolerance : UFix64 < N4 >) -> SlippageConfig;
    pub fn expected_token_out< Exp : Integer >(& self) -> Result < UFix64 < Exp > >;
    pub fn slippage_tolerance(& self) -> Result < UFix64 < N4 > >;
    fn validate_inner< Exp >(expected : UFix64 < Exp >, tolerance : UFix64 < N4 >, token_out : UFix64 < Exp >) -> Result < () >;
    pub fn validate_token_out< Exp : Integer >(& self, token_out : UFix64 < Exp >) -> Result < () >;
    pub fn validate_token_out_normalized(& self, mint : & Mint, token_out : UFix64 < N9 >) -> Result < () >;
}


---

# crate::solana_clock
<!-- file: hylo-core/src/solana_clock.rs -->

## Traits

/// Abstracts the concept of Solana's onchain clock.
// NOTE: Abstraction trait over Solana's Clock sysvar providing slot, epoch, and timestamp access.
pub trait SolanaClock {
    fn slot(& self) -> u64;
    fn epoch_start_timestamp(& self) -> i64;
    fn epoch(& self) -> u64;
    fn leader_schedule_epoch(& self) -> u64;
    fn unix_timestamp(& self) -> i64;
}


## Impl SolanaClock for Clock

// NOTE: Implements SolanaClock for Solana's native Clock type.
impl SolanaClock for Clock {
    fn slot(& self) -> u64;
    fn epoch_start_timestamp(& self) -> i64;
    fn epoch(& self) -> u64;
    fn leader_schedule_epoch(& self) -> u64;
    fn unix_timestamp(& self) -> i64;
}


## Impl SolanaClock for ClockRef

// NOTE: Implements SolanaClock for the serializable ClockRef wrapper.
impl SolanaClock for ClockRef {
    fn slot(& self) -> u64;
    fn epoch_start_timestamp(& self) -> i64;
    fn epoch(& self) -> u64;
    fn leader_schedule_epoch(& self) -> u64;
    fn unix_timestamp(& self) -> i64;
}


---

# crate::spl_stake_pool
<!-- file: hylo-core/src/spl_stake_pool.rs -->

## Types

/// Minimal view of stake pool PDA used in all SPL LST programs.
pub struct SplStakePool {
    pub total_lamports: UFix64 < N9 >,
    pub pool_token_supply: UFix64 < N9 >,
    pub last_update_epoch: u64,
}


## Impl SplStakePool

impl SplStakePool {
    pub fn from_bytes(data : & [u8]) -> Result < SplStakePool >;
    pub fn true_price(& self) -> Result < LstSolPrice >;
}


## Constants

/// Byte offsets in [`StakePool`].
/// <https://docs.rs/spl-stake-pool/latest/spl_stake_pool/state/struct.StakePool.html>
const TOTAL_LAMPORTS_OFFSET: usize;

const POOL_TOKEN_SUPPLY_OFFSET: usize;

const LAST_UPDATE_EPOCH_OFFSET: usize;

const U64_SIZE: usize;


---

# crate::stability_mode
<!-- file: hylo-core/src/stability_mode.rs -->

## Types

/// Mode of operation based on the protocol's current collateral ratio.
/// See whitepaper for more.
// NOTE: Protocol operating mode (Normal, Mode1, Mode2, Depeg) determined by collateral ratio vs thresholds.
pub enum StabilityMode {
    Normal,
    Mode1,
    Mode2,
    Depeg,
}

// NOTE: Holds collateral ratio thresholds and determines the current stability mode.
pub struct StabilityController {
    pub stability_threshold_1: UFix64 < N2 >,
    pub stability_threshold_2: UFix64 < N2 >,
    pub min_stability_threshold: UFix64 < N2 >,
}


## Impl Display for StabilityMode

// NOTE: Human-readable display formatting for stability mode variants.
impl Display for StabilityMode {
    fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt :: Result;
}


## Impl StabilityController

// NOTE: Holds collateral ratio thresholds and determines the current stability mode.
impl StabilityController {
    pub fn new(stability_threshold_1 : UFix64 < N2 >, stability_threshold_2 : UFix64 < N2 >) -> Result < StabilityController >;
    pub fn stability_mode(& self, collateral_ratio : UFix64 < N9 >) -> Result < StabilityMode >;
    pub fn prev_stability_threshold(& self, mode : StabilityMode) -> Option < UFix64 < N2 > >;
    pub fn next_stability_threshold(& self, mode : StabilityMode) -> Option < UFix64 < N2 > >;
    pub fn min_stability_threshold(& self) -> UFix64 < N2 >;
    pub fn validate(& self) -> Result < () >;
}


---

# crate::stability_pool_math
<!-- file: hylo-core/src/stability_pool_math.rs -->

## Functions

/// Computes NAV for the stability pool's LP token.
/// 
/// ```txt
///                  stablecoin_in_pool
/// lp_token_nav =  --------------------
///                   lp_token_supply
/// ```
// NOTE: Computes sHYUSD LP token NAV as stability_pool_cap / lp_token_supply.
pub fn lp_token_nav(stablecoin_in_pool : UFix64 < N6 >, lp_token_supply : UFix64 < N6 >) -> Result < UFix64 < N6 > >;

/// Simply divides the amount of stablecoin being deposited by the LP token NAV.
// NOTE: Computes LP tokens minted for a given stablecoin deposit amount.
pub fn lp_token_out(amount_stablecoin_in : UFix64 < N6 >, lp_token_nav : UFix64 < N6 >) -> Result < UFix64 < N6 > >;

/// Computes amount of token to withdraw, given a user's LP equity in the pool.
// NOTE: Computes a user's proportional share of a pool token based on their LP token holdings.
pub fn amount_token_to_withdraw(user_lp_token_amount : UFix64 < N6 >, lp_token_supply : UFix64 < N6 >, pool_amount : UFix64 < N6 >) -> Result < UFix64 < N6 > >;

/// Computes a stablecoin target based on levercoin in pool.
/// Compares to max mintable stablecoin and returns lesser of the two.
// NOTE: Computes stablecoin amount to swap from levercoin in pool, capped by max_swappable.
pub fn amount_lever_to_swap(levercoin_in_pool : UFix64 < N6 >, levercoin_nav : PriceRange < N9 >, max_swappable_stablecoin : UFix64 < N6 >) -> Result < UFix64 < N6 > >;

/// Extracts withdrawal fee from stablecoin amount.
// NOTE: Extracts single-sided withdrawal fees in stablecoin from a user's proportional allocation.
pub fn stablecoin_withdrawal_fee(stablecoin_to_withdraw : UFix64 < N6 >, withdrawal_fee : UFix64 < N4 >) -> Result < FeeExtract < N6 > >;


---

# crate::total_sol_cache
<!-- file: hylo-core/src/total_sol_cache.rs -->

## Types

// NOTE: Epoch-validated running total of SOL collateral, updated on each deposit/withdrawal.
pub struct TotalSolCache {
    pub current_update_epoch: u64,
    pub total_sol: UFixValue64,
}


## Impl TotalSolCache

// NOTE: Epoch-validated running total of SOL collateral, updated on each deposit/withdrawal.
impl TotalSolCache {
    pub fn new(current_update_epoch : u64) -> TotalSolCache;
    pub fn increment(& mut self, sol_in : UFix64 < N9 >, current_epoch : u64) -> Result < () >;
    pub fn decrement(& mut self, sol_out : UFix64 < N9 >, current_epoch : u64) -> Result < () >;
    pub fn set(& mut self, total_sol : UFix64 < N9 >, current_epoch : u64) -> Result < () >;
    pub fn get_validated(& self, current_epoch : u64) -> Result < UFix64 < N9 > >;
}


---

# crate::util
<!-- file: hylo-core/src/util.rs -->

## Functions

/// Bridges runtime mint decimals to typed `UFix64<N9>`.
/// 
/// # Errors
/// * Unsupported decimal count or conversion overflow
pub fn normalize_mint_exp(mint : & Mint, amount : u64) -> Result < UFix64 < N9 > >;

/// Converts typed `UFix64<N9>` back to a raw `u64` in the mint's native
/// decimals.
/// 
/// # Errors
/// * Unsupported decimal count
pub fn denormalize_mint_exp(mint : & Mint, amount : UFix64 < N9 >) -> Result < u64 >;


## Macros

// NOTE: Assertion macro checking two fixed-point values are within a specified tolerance.
macro_rules! eq_tolerance { ... }


---

# crate::virtual_stablecoin
<!-- file: hylo-core/src/virtual_stablecoin.rs -->

## Types

/// Simple counter representing the supply of a "virtual" stablecoin.
// NOTE: Counter tracking virtual stablecoin supply for exo pairs that don't have a real SPL mint.
pub struct VirtualStablecoin {
    pub supply: UFixValue64,
}


## Impl Default for VirtualStablecoin

// NOTE: Initializes VirtualStablecoin with zero supply.
impl Default for VirtualStablecoin {
    fn default() -> Self;
}


## Impl VirtualStablecoin

// NOTE: Counter tracking virtual stablecoin supply for exo pairs that don't have a real SPL mint.
impl VirtualStablecoin {
    pub fn new() -> VirtualStablecoin;
    pub fn supply(& self) -> Result < UFix64 < N6 > >;
    pub fn mint(& mut self, amount : UFix64 < N6 >) -> Result < () >;
    pub fn burn(& mut self, amount : UFix64 < N6 >) -> Result < () >;
}


---

# crate::yields
<!-- file: hylo-core/src/yields.rs -->

## Types

/// Captures yield harvest configuration as two basis point values:
// NOTE: Configures what fraction of LST yield is harvested and what fee is taken from harvested yield.
pub struct YieldHarvestConfig {
    pub allocation: UFixValue64,
    pub fee: UFixValue64,
}

/// Records epoch harvest information for off-chain consumers.
// NOTE: Records the last epoch's yield harvest: pool cap and stablecoin amount distributed.
pub struct HarvestCache {
    pub epoch: u64,
    pub stability_pool_cap: UFixValue64,
    pub stablecoin_to_pool: UFixValue64,
}


## Impl YieldHarvestConfig

// NOTE: Configures what fraction of LST yield is harvested and what fee is taken from harvested yield.
impl YieldHarvestConfig {
    pub fn init(& mut self, allocation : UFixValue64, fee : UFixValue64) -> Result < () >;
    pub fn allocation(& self) -> Result < UFix64 < N4 > >;
    pub fn fee(& self) -> Result < UFix64 < N4 > >;
    pub fn apply_allocation(& self, stablecoin : UFix64 < N6 >) -> Result < UFix64 < N6 > >;
    pub fn apply_fee(& self, stablecoin : UFix64 < N6 >) -> Result < FeeExtract < N6 > >;
    pub fn validate(& self) -> Result < Self >;
}


## Impl HarvestCache

// NOTE: Records the last epoch's yield harvest: pool cap and stablecoin amount distributed.
impl HarvestCache {
    pub fn init(& mut self, epoch : u64) -> Result < () >;
    pub fn update(& mut self, stability_pool_cap : UFix64 < N6 >, stablecoin_to_pool : UFix64 < N6 >, epoch : u64) -> Result < () >;
    pub fn is_stale(& self, current_epoch : u64) -> bool;
}


## Constants

/// 1000 bps (10%)
const MAX_FEE: UFix64 < N4 >;


---

# Crate: hylo_idl (lib)

# crate
<!-- file: hylo-idl/src/lib.rs -->

---

# crate::codegen
<!-- file: hylo-idl/src/lib.rs -->

---

# crate::account_builders
<!-- file: hylo-idl/src/account_builders/mod.rs -->

---

# crate::account_builders::exchange
<!-- file: hylo-idl/src/account_builders/exchange.rs -->

## Functions

/// Builds account context for stablecoin mint (LST -> hyUSD).
pub fn mint_stablecoin_lst(user : Pubkey, lst_mint : Pubkey) -> MintStablecoinLst;

/// Builds account context for levercoin mint (LST -> xSOL).
pub fn mint_levercoin_lst(user : Pubkey, lst_mint : Pubkey) -> MintLevercoinLst;

/// Builds account context for stablecoin redemption (hyUSD -> LST).
pub fn redeem_stablecoin_lst(user : Pubkey, lst_mint : Pubkey) -> RedeemStablecoinLst;

/// Builds account context for levercoin redemption (xSOL -> LST).
pub fn redeem_levercoin_lst(user : Pubkey, lst_mint : Pubkey) -> RedeemLevercoinLst;

/// Builds account context for stable-to-lever convert (hyUSD -> xSOL).
pub fn convert_stable_to_lever_lst(user : Pubkey) -> ConvertStableToLeverLst;

/// Builds account context for lever-to-stable convert (xSOL -> hyUSD).
pub fn convert_lever_to_stable_lst(user : Pubkey) -> ConvertLeverToStableLst;

/// Builds account context for registering an EXO pair.
// NOTE: Builds the Anchor account context for registering a new exogenous collateral pair.
pub fn register_exo(admin : Pubkey, collateral_mint : Pubkey, exo_usd_pyth_feed : Pubkey) -> RegisterExo;

/// Exo levercoin mint (collateral -> exo levercoin).
pub fn mint_levercoin_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> MintLevercoinExo;

/// Exo stablecoin mint (collateral -> hyUSD).
pub fn mint_stablecoin_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> MintStablecoinExo;

/// Exo levercoin redemption (exo levercoin -> collateral).
pub fn redeem_levercoin_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> RedeemLevercoinExo;

/// Exo stablecoin redemption (hyUSD -> collateral).
pub fn redeem_stablecoin_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> RedeemStablecoinExo;

/// Builds account context for harvesting exo funding rate.
pub fn harvest_funding_rate(collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> HarvestFundingRate;

/// Lever-to-stable convert exo (xAsset -> hyUSD).
pub fn convert_lever_to_stable_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> ConvertLeverToStableExo;

/// Stable-to-lever convert exo (hyUSD -> xAsset).
pub fn convert_stable_to_lever_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> ConvertStableToLeverExo;

/// Builds account context for withdrawing protocol fees.
pub fn withdraw_fees(payer : Pubkey, treasury : Pubkey, fee_token_mint : Pubkey) -> WithdrawFees;

/// Builds account context for LST-to-LST swap.
pub fn swap_lst_to_lst(user : Pubkey, lst_a : Pubkey, lst_b : Pubkey) -> SwapLstToLst;

/// Exo collateral to USDC swap.
pub fn swap_exo_to_usdc(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> SwapExoToUsdc;

/// USDC to exo collateral swap.
pub fn swap_usdc_to_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> SwapUsdcToExo;

/// LST to USDC swap.
pub fn swap_lst_to_usdc(user : Pubkey, lst_mint : Pubkey, pool_state : Pubkey) -> SwapLstToUsdc;

/// USDC to LST swap.
pub fn swap_usdc_to_lst(user : Pubkey, lst_mint : Pubkey, pool_state : Pubkey) -> SwapUsdcToLst;

/// Builds account context for initializing the USDC pair.
pub fn initialize_usdc(admin : Pubkey, usdc_usd_pyth_feed : Pubkey) -> InitializeUsdc;

/// Builds account context for hyUSD mint from USDC.
pub fn mint_stablecoin_usdc(user : Pubkey) -> MintStablecoinUsdc;

/// Builds account context for hyUSD redeem to USDC.
pub fn redeem_stablecoin_usdc(user : Pubkey) -> RedeemStablecoinUsdc;

pub fn update_lst_rebalance_fee(admin : Pubkey, lst_mint : Pubkey) -> UpdateLstRebalanceFee;


---

# crate::account_builders::stability_pool
<!-- file: hylo-idl/src/account_builders/stability_pool.rs -->

## Functions

/// Builds account context for stability pool deposit (hyUSD -> sHYUSD).
// NOTE: Builds the Anchor account context for depositing hyUSD into the stability pool for sHYUSD.
pub fn deposit(user : Pubkey) -> UserDeposit;

/// Builds account context for stability pool withdrawal (sHYUSD -> hyUSD).
// NOTE: Builds the Anchor account context for withdrawing sHYUSD from the stability pool.
pub fn withdraw(user : Pubkey) -> UserWithdraw;


---

# crate::instruction_builders
<!-- file: hylo-idl/src/instruction_builders/mod.rs -->

---

# crate::instruction_builders::exchange
<!-- file: hylo-idl/src/instruction_builders/exchange.rs -->

## Functions

pub fn mint_stablecoin_lst(user : Pubkey, lst_mint : Pubkey, args : & args :: MintStablecoinLst) -> Instruction;

pub fn mint_levercoin_lst(user : Pubkey, lst_mint : Pubkey, args : & args :: MintLevercoinLst) -> Instruction;

pub fn redeem_stablecoin_lst(user : Pubkey, lst_mint : Pubkey, args : & args :: RedeemStablecoinLst) -> Instruction;

pub fn redeem_levercoin_lst(user : Pubkey, lst_mint : Pubkey, args : & args :: RedeemLevercoinLst) -> Instruction;

pub fn convert_stable_to_lever_lst(user : Pubkey, args : & args :: ConvertStableToLeverLst) -> Instruction;

pub fn convert_lever_to_stable_lst(user : Pubkey, args : & args :: ConvertLeverToStableLst) -> Instruction;

// NOTE: Builds the instruction to initialize the Hylo exchange program with admin settings.
pub fn initialize_protocol(admin : Pubkey, upgrade_authority : Pubkey, treasury : Pubkey, args : & args :: InitializeProtocol) -> Instruction;

// NOTE: Builds the instruction to create hyUSD, xSOL, and related token mint accounts.
pub fn initialize_mints(admin : Pubkey, stablecoin_metadata : TokenMetadata, levercoin_metadata : TokenMetadata) -> Instruction;

// NOTE: Builds the instruction to create a new LST registry account for tracking supported LSTs.
pub fn initialize_lst_registry(slot : u64, admin : Pubkey) -> Instruction;

// NOTE: Builds the instruction to initialize Sanctum calculator references in the LST registry.
pub fn initialize_lst_registry_calculators(lst_registry : Pubkey, admin : Pubkey) -> Instruction;

// NOTE: Builds the register_lst instruction to add a new LST to the registry with its Sanctum calculator.
pub fn register_lst(lst_mint : Pubkey, lst_stake_pool_state : Pubkey, sanctum_calculator_program : Pubkey, sanctum_calculator_state : Pubkey, stake_pool_program : Pubkey, stake_pool_program_data : Pubkey, lst_registry : Pubkey, admin : Pubkey, rebalance_fee : UFixValue64) -> Instruction;

pub fn update_lst_rebalance_fee(admin : Pubkey, lst_mint : Pubkey, args : & args :: UpdateLstRebalanceFee) -> Instruction;

// NOTE: Builds the admin instruction to update the Pyth oracle confidence tolerance.
pub fn update_oracle_conf_tolerance(admin : Pubkey, args : & args :: UpdateOracleConfTolerance) -> Instruction;

// NOTE: Builds the admin instruction to update the SOL/USD oracle configuration.
pub fn update_sol_usd_oracle(admin : Pubkey, args : & args :: UpdateSolUsdOracle) -> Instruction;

// NOTE: Builds the admin instruction to update the stability pool address in the exchange.
pub fn update_stability_pool(admin : Pubkey, args : & args :: UpdateStabilityPool) -> Instruction;

// NOTE: Builds the harvest_yield instruction to collect LST yield and distribute to the stability pool.
pub fn harvest_yield(lst_registry : Pubkey, remaining_accounts : Vec < AccountMeta >) -> Instruction;

// NOTE: Builds the instruction to refresh all LST-SOL prices from the Sanctum calculators.
pub fn update_lst_prices(payer : Pubkey, lst_registry : Pubkey, remaining_accounts : Vec < AccountMeta >) -> Instruction;

pub fn swap_lst_to_lst(user : Pubkey, lst_a : Pubkey, lst_b : Pubkey, args : & args :: SwapLstToLst) -> Instruction;

// NOTE: Builds the register_exo instruction to add a new exogenous collateral pair.
pub fn register_exo(admin : Pubkey, collateral_mint : Pubkey, exo_usd_pyth_feed : Pubkey, args : & args :: RegisterExo) -> Instruction;

pub fn mint_levercoin_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey, args : & args :: MintLevercoinExo) -> Instruction;

pub fn mint_stablecoin_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey, args : & args :: MintStablecoinExo) -> Instruction;

pub fn redeem_levercoin_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey, args : & args :: RedeemLevercoinExo) -> Instruction;

pub fn redeem_stablecoin_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey, args : & args :: RedeemStablecoinExo) -> Instruction;

pub fn harvest_funding_rate(collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey) -> Instruction;

pub fn convert_lever_to_stable_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey, args : & args :: ConvertLeverToStableExo) -> Instruction;

pub fn convert_stable_to_lever_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey, args : & args :: ConvertStableToLeverExo) -> Instruction;

// NOTE: Builds the admin instruction to update the fee for LST-to-LST swaps.
pub fn update_lst_swap_fee(admin : Pubkey, args : & args :: UpdateLstSwapFee) -> Instruction;

pub fn update_levercoin_fees(admin : Pubkey, args : & args :: UpdateLevercoinFees) -> Instruction;

pub fn update_oracle_interval(admin : Pubkey, args : & args :: UpdateOracleInterval) -> Instruction;

pub fn update_stability_thresholds(admin : Pubkey, args : & args :: UpdateStabilityThresholds) -> Instruction;

pub fn update_paused(admin : Pubkey, args : & args :: UpdatePaused) -> Instruction;

pub fn update_lst_buy_curve_config(admin : Pubkey, args : & args :: UpdateLstBuyCurveConfig) -> Instruction;

pub fn update_lst_sell_curve_config(admin : Pubkey, args : & args :: UpdateLstSellCurveConfig) -> Instruction;

pub fn update_treasury(admin : Pubkey, args : & args :: UpdateTreasury) -> Instruction;

pub fn update_yield_harvest_config(admin : Pubkey, args : & args :: UpdateYieldHarvestConfig) -> Instruction;

pub fn update_exo_funding_rate(admin : Pubkey, collateral_mint : Pubkey, args : & args :: UpdateExoFundingRate) -> Instruction;

pub fn update_exo_oracle(admin : Pubkey, collateral_mint : Pubkey, args : & args :: UpdateExoOracle) -> Instruction;

pub fn update_exo_oracle_conf_tolerance(admin : Pubkey, collateral_mint : Pubkey, args : & args :: UpdateExoOracleConfTolerance) -> Instruction;

pub fn update_exo_oracle_interval(admin : Pubkey, collateral_mint : Pubkey, args : & args :: UpdateExoOracleInterval) -> Instruction;

pub fn update_exo_stability_threshold(admin : Pubkey, collateral_mint : Pubkey, args : & args :: UpdateExoStabilityThreshold) -> Instruction;

pub fn update_exo_buy_curve(admin : Pubkey, collateral_mint : Pubkey, args : & args :: UpdateExoBuyCurve) -> Instruction;

pub fn update_exo_sell_curve(admin : Pubkey, collateral_mint : Pubkey, args : & args :: UpdateExoSellCurve) -> Instruction;

pub fn update_exo_levercoin_fees(admin : Pubkey, collateral_mint : Pubkey, args : & args :: UpdateExoLevercoinFees) -> Instruction;

pub fn update_admin(payer : Pubkey, upgrade_authority : Pubkey, args : & args :: UpdateAdmin) -> Instruction;

pub fn initialize_usdc(admin : Pubkey, usdc_usd_pyth_feed : Pubkey, args : & args :: InitializeUsdc) -> Instruction;

pub fn redeem_stablecoin_usdc(user : Pubkey, args : & args :: RedeemStablecoinUsdc) -> Instruction;

pub fn mint_stablecoin_usdc(user : Pubkey, args : & args :: MintStablecoinUsdc) -> Instruction;

pub fn update_usdc_oracle_conf_tolerance(admin : Pubkey, args : & args :: UpdateUsdcOracleConfTolerance) -> Instruction;

pub fn update_usdc_oracle_interval(admin : Pubkey, args : & args :: UpdateUsdcOracleInterval) -> Instruction;

pub fn update_usdc_swap_fee(admin : Pubkey, args : & args :: UpdateUsdcSwapFee) -> Instruction;

pub fn initialize_lst_virtual_stablecoin(admin : Pubkey) -> Instruction;

pub fn swap_exo_to_usdc(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey, args : & args :: SwapExoToUsdc) -> Instruction;

pub fn swap_usdc_to_exo(user : Pubkey, collateral_mint : Pubkey, collateral_usd_pyth_feed : Pubkey, args : & args :: SwapUsdcToExo) -> Instruction;

pub fn swap_lst_to_usdc(user : Pubkey, lst_mint : Pubkey, pool_state : Pubkey, args : & args :: SwapLstToUsdc) -> Instruction;

pub fn swap_usdc_to_lst(user : Pubkey, lst_mint : Pubkey, pool_state : Pubkey, args : & args :: SwapUsdcToLst) -> Instruction;

pub fn get_stats() -> Instruction;

pub fn withdraw_fees(payer : Pubkey, treasury : Pubkey, fee_token_mint : Pubkey) -> Instruction;


---

# crate::instruction_builders::router
<!-- file: hylo-idl/src/instruction_builders/router.rs -->

## Functions

/// Routes through the proxy program, forwarding the given accounts
/// to the target program via CPI.
pub fn route< A : ToAccountMetas >(args : & args :: Route, inner_accounts : & A) -> Instruction;


---

# crate::instruction_builders::stability_pool
<!-- file: hylo-idl/src/instruction_builders/stability_pool.rs -->

## Functions

// NOTE: Builds the instruction for a user to deposit hyUSD into the stability pool.
pub fn user_deposit(user : Pubkey, args : & args :: UserDeposit) -> Instruction;

// NOTE: Builds the instruction for a user to withdraw sHYUSD from the stability pool.
pub fn user_withdraw(user : Pubkey, args : & args :: UserWithdraw) -> Instruction;

// NOTE: Builds the instruction to rebalance pool by swapping levercoin for stablecoin.
pub fn rebalance_lever_to_stable(payer : Pubkey) -> Instruction;

// NOTE: Builds the instruction to query stability pool statistics via simulation.
pub fn get_stats() -> Instruction;

// NOTE: Builds the instruction to initialize the stability pool program.
pub fn initialize_stability_pool(admin : Pubkey, upgrade_authority : Pubkey) -> Instruction;

// NOTE: Builds the instruction to create the sHYUSD LP token mint.
pub fn initialize_lp_token_mint(admin : Pubkey, lp_token_metadata : TokenMetadata) -> Instruction;

// NOTE: Builds the admin instruction to update the stability pool withdrawal fee.
pub fn update_withdrawal_fee(admin : Pubkey, args : & args :: UpdateWithdrawalFee) -> Instruction;

pub fn update_admin(payer : Pubkey, upgrade_authority : Pubkey, args : & args :: UpdateAdmin) -> Instruction;


---

# crate::exchange
<!-- file: hylo-idl/src/lib.rs -->

## Constants

pub const ID: Pubkey;


## Re-exports

// NOTE: Re-export of exchange account builders into the exchange module namespace.
pub use super :: account_builders :: exchange as account_builders;

// NOTE: Glob re-export of Anchor-generated exchange program types and instructions.
pub use super :: codegen :: hylo_exchange :: *;

// NOTE: Re-export of exchange instruction builders into the exchange module namespace.
pub use super :: instruction_builders :: exchange as instruction_builders;


---

# crate::stability_pool
<!-- file: hylo-idl/src/lib.rs -->

## Constants

pub const ID: Pubkey;


## Re-exports

// NOTE: Re-export of stability pool account builders into the stability_pool module namespace.
pub use super :: account_builders :: stability_pool as account_builders;

// NOTE: Glob re-export of Anchor-generated stability pool program types and instructions.
pub use super :: codegen :: hylo_stability_pool :: *;

// NOTE: Re-export of stability pool instruction builders into the stability_pool module namespace.
pub use super :: instruction_builders :: stability_pool as instruction_builders;


---

# crate::router
<!-- file: hylo-idl/src/lib.rs -->

## Constants

pub const ID: Pubkey;


## Re-exports

pub use super :: codegen :: hylo_router :: *;

pub use super :: instruction_builders :: router as instruction_builders;


---

# crate::pda
<!-- file: hylo-idl/src/pda.rs -->

## Functions

pub const fn mint< const N : usize >(program_id : Pubkey, seed : [u8 ; N]) -> Pubkey;

// NOTE: Macro for deriving Associated Token Account addresses.
pub const fn ata(auth : Pubkey, mint : Pubkey) -> Pubkey;

pub const fn progdata(program_id : Pubkey) -> Pubkey;

// NOTE: Derives the Metaplex token metadata PDA for a given mint.
pub const fn metadata(mint : Pubkey) -> Pubkey;

// NOTE: Derives the hyUSD Associated Token Account for a given authority.
pub const fn hyusd_ata(auth : Pubkey) -> Pubkey;

// NOTE: Derives the xSOL Associated Token Account for a given authority.
pub const fn xsol_ata(auth : Pubkey) -> Pubkey;

// NOTE: Derives the sHYUSD Associated Token Account for a given authority.
pub const fn shyusd_ata(auth : Pubkey) -> Pubkey;

pub const fn usdc_ata(auth : Pubkey) -> Pubkey;

pub const fn lst_vault(mint : Pubkey) -> Pubkey;

pub const fn exo_vault(mint : Pubkey) -> Pubkey;

pub const fn usdc_vault(mint : Pubkey) -> Pubkey;

pub const fn lst_vault_auth(mint : Pubkey) -> Pubkey;

pub const fn exo_vault_auth(mint : Pubkey) -> Pubkey;

pub const fn usdc_vault_auth(mint : Pubkey) -> Pubkey;

// NOTE: Derives the PDA for an LST registry keyed by creation slot.
pub const fn new_lst_registry(slot : u64) -> Pubkey;

// NOTE: Derives the PDA for an LST's header account in the registry.
pub const fn lst_header(mint : Pubkey) -> Pubkey;

// NOTE: Derives the PDA for a token's fee vault account.
pub const fn fee_vault(mint : Pubkey) -> Pubkey;

// NOTE: Derives the PDA for a token's fee authority.
pub const fn fee_auth(mint : Pubkey) -> Pubkey;

// NOTE: Derives the PDA for a token's mint authority.
pub const fn mint_auth(mint : Pubkey) -> Pubkey;

pub const fn event_auth(program_id : Pubkey) -> Pubkey;

// NOTE: Derives the PDA for an exogenous collateral pair's state account.
pub const fn exo_pair(collateral_mint : Pubkey) -> Pubkey;

// NOTE: Derives the PDA for an exogenous pair's levercoin mint given its collateral mint.
pub const fn exo_levercoin_mint(collateral_mint : Pubkey) -> Pubkey;


## Constants

// NOTE: Lazily-derived PDA for the main Hylo protocol state account.
pub const HYLO: Pubkey;

// NOTE: Lazily-derived PDA for the hyUSD mint authority.
pub const HYUSD_AUTH: Pubkey;

// NOTE: Lazily-derived PDA for the xSOL mint authority.
pub const XSOL_AUTH: Pubkey;

// NOTE: Lazily-derived PDA for the LST registry authority.
pub const LST_REGISTRY_AUTH: Pubkey;

// NOTE: Lazily-derived PDA for the stability pool configuration account.
pub const POOL_CONFIG: Pubkey;

// NOTE: Lazily-derived PDA for the sHYUSD mint authority.
pub const SHYUSD_AUTH: Pubkey;

// NOTE: Lazily-derived PDA for the stability pool authority.
pub const POOL_AUTH: Pubkey;

// NOTE: Lazily-derived PDA for the hyUSD token account in the stability pool.
pub const HYUSD_POOL: Pubkey;

// NOTE: Lazily-derived PDA for the xSOL token account in the stability pool.
pub const XSOL_POOL: Pubkey;

// NOTE: Lazily-derived PDA for the stability pool program's data account.
pub const STABILITY_POOL_PROGRAM_DATA: Pubkey;

// NOTE: Lazily-derived PDA for the exchange program's data account.
pub const EXCHANGE_PROGRAM_DATA: Pubkey;

// NOTE: Hard-coded Pubkey of the Pyth SOL/USD price feed account.
pub const SOL_USD_PYTH_FEED: Pubkey;

pub const USDC_USD_PYTH_FEED: Pubkey;

pub const BTC_USD_PYTH_FEED: Pubkey;

pub const EXCHANGE_EVENT_AUTHORITY: Pubkey;

pub const STABILITY_POOL_EVENT_AUTHORITY: Pubkey;

pub const USDC_PAIR: Pubkey;


## Macros

// NOTE: Macro for deriving program-derived addresses with seeds.
macro_rules! pda { ... }


---

# crate::tokens
<!-- file: hylo-idl/src/tokens.rs -->

## Types

// NOTE: Type-safe marker for the hyUSD stablecoin (N6 precision).
pub struct HYUSD;

// NOTE: Type-safe marker for the sHYUSD stability pool LP token (N6 precision).
pub struct SHYUSD;

// NOTE: Type-safe marker for the xSOL leverage token (N6 precision).
pub struct XSOL;

// NOTE: Type-safe marker for the JITOSOL liquid staking token (N9 precision).
pub struct JITOSOL;

// NOTE: Type-safe marker for the HYLOSOL liquid staking token (N9 precision).
pub struct HYLOSOL;

pub struct USDC;

pub struct CBBTC;

pub struct XBTC;


## Traits

// NOTE: Trait associating a token with its mint Pubkey and decimal precision exponent type.
pub trait TokenMint {
    type Exp: Integer;
    const MINT: Pubkey;
}

pub trait StakePool: TokenMint < Exp = N9 > {
    const POOL_STATE: Pubkey;
}


## Impl TokenMint for HYUSD

// NOTE: Associates HYUSD with its mint address and N6 exponent.
impl TokenMint for HYUSD {
    type Exp = N6;
    const MINT: Pubkey;
}


## Impl TokenMint for SHYUSD

// NOTE: Associates SHYUSD with its mint address and N6 exponent.
impl TokenMint for SHYUSD {
    type Exp = N6;
    const MINT: Pubkey;
}


## Impl TokenMint for XSOL

// NOTE: Associates XSOL with its mint address and N6 exponent.
impl TokenMint for XSOL {
    type Exp = N6;
    const MINT: Pubkey;
}


## Impl TokenMint for JITOSOL

// NOTE: Associates JITOSOL with its mint address and N9 exponent.
impl TokenMint for JITOSOL {
    type Exp = N9;
    const MINT: Pubkey;
}


## Impl StakePool for JITOSOL

impl StakePool for JITOSOL {
    const POOL_STATE: Pubkey;
}


## Impl TokenMint for HYLOSOL

// NOTE: Associates HYLOSOL with its mint address and N9 exponent.
impl TokenMint for HYLOSOL {
    type Exp = N9;
    const MINT: Pubkey;
}


## Impl StakePool for HYLOSOL

impl StakePool for HYLOSOL {
    const POOL_STATE: Pubkey;
}


## Impl TokenMint for USDC

impl TokenMint for USDC {
    type Exp = N6;
    const MINT: Pubkey;
}


## Impl TokenMint for CBBTC

impl TokenMint for CBBTC {
    type Exp = N8;
    const MINT: Pubkey;
}


## Impl TokenMint for XBTC

impl TokenMint for XBTC {
    type Exp = N6;
    const MINT: Pubkey;
}


---

# crate::type_bridge
<!-- file: hylo-idl/src/type_bridge.rs -->

## Impl From < crate :: exchange :: types :: UFixValue64 > for UFixValue64

// NOTE: Converts exchange IDL UFixValue64 into hylo-fix UFixValue64.
impl From < crate :: exchange :: types :: UFixValue64 > for UFixValue64 {
    fn from(idl : crate :: exchange :: types :: UFixValue64) -> Self;
}


## Impl TryFrom < crate :: exchange :: types :: UFixValue64 > for UFix64 < Exp >

// NOTE: Fallibly converts exchange IDL UFixValue64 into a typed UFix64 with compile-time exponent.
impl < Exp : Integer >TryFrom < crate :: exchange :: types :: UFixValue64 > for UFix64 < Exp > {
    type Error = anchor_lang :: error :: Error;
    fn try_from(idl : crate :: exchange :: types :: UFixValue64) -> Result < Self , Self :: Error >;
}


## Impl From < crate :: stability_pool :: types :: UFixValue64 > for UFixValue64

// NOTE: Converts stability pool IDL UFixValue64 into hylo-fix UFixValue64.
impl From < crate :: stability_pool :: types :: UFixValue64 > for UFixValue64 {
    fn from(idl : crate :: stability_pool :: types :: UFixValue64) -> Self;
}


## Impl TryFrom < crate :: stability_pool :: types :: UFixValue64 > for UFix64 < Exp >

// NOTE: Fallibly converts stability pool IDL UFixValue64 into a typed UFix64 with compile-time exponent.
impl < Exp : Integer >TryFrom < crate :: stability_pool :: types :: UFixValue64 > for UFix64 < Exp > {
    type Error = anchor_lang :: error :: Error;
    fn try_from(idl : crate :: stability_pool :: types :: UFixValue64) -> Result < Self , Self :: Error >;
}


## Impl From < UFixValue64 > for crate :: exchange :: types :: UFixValue64

// NOTE: Converts hylo-fix UFixValue64 into the exchange program's IDL UFixValue64.
impl From < UFixValue64 > for crate :: exchange :: types :: UFixValue64 {
    fn from(idl : UFixValue64) -> Self;
}


## Impl From < UFixValue64 > for crate :: router :: types :: UFixValue64

impl From < UFixValue64 > for crate :: router :: types :: UFixValue64 {
    fn from(val : UFixValue64) -> Self;
}


## Impl From < crate :: exchange :: types :: SlippageConfig > for crate :: router :: types :: SlippageConfig

impl From < crate :: exchange :: types :: SlippageConfig > for crate :: router :: types :: SlippageConfig {
    fn from(val : crate :: exchange :: types :: SlippageConfig) -> Self;
}


---

# Crate: lut-accounts (bin)

# crate
<!-- file: hylo-idl/src/bin/lut_accounts.rs -->

## Functions

fn main();


## Constants

const LUT_ACCOUNTS: & [Pubkey];


---

# Crate: hylo_jupiter (lib)

# crate
<!-- file: hylo-jupiter/src/lib.rs -->

## Re-exports

// NOTE: Re-exports HyloJupiterPair and PairConfig from the jupiter module.
pub use jupiter :: { HyloJupiterPair , PairConfig };


---

# crate::account_metas
<!-- file: hylo-jupiter/src/account_metas.rs -->

## Functions

fn route_account_metas< A : ToAccountMetas >(in_token : Pubkey, out_token : Pubkey, inner_accounts : & A) -> SwapAndAccountMetas;

/// Mint hyUSD from LST.
pub fn mint_stablecoin_lst(user : Pubkey, lst_mint : Pubkey) -> SwapAndAccountMetas;

/// Mint xSOL from LST.
pub fn mint_levercoin_lst(user : Pubkey, lst_mint : Pubkey) -> SwapAndAccountMetas;

/// Redeem hyUSD for LST.
pub fn redeem_stablecoin_lst(user : Pubkey, lst_mint : Pubkey) -> SwapAndAccountMetas;

/// Redeem xSOL for LST.
pub fn redeem_levercoin_lst(user : Pubkey, lst_mint : Pubkey) -> SwapAndAccountMetas;

/// Convert hyUSD to xSOL.
pub fn convert_stable_to_lever_lst(user : Pubkey) -> SwapAndAccountMetas;

/// Convert xSOL to hyUSD.
pub fn convert_lever_to_stable_lst(user : Pubkey) -> SwapAndAccountMetas;

/// Deposit hyUSD to stability pool.
// NOTE: Creates Jupiter-compatible SwapAndAccountMetas for depositing hyUSD into the stability pool.
pub fn stability_pool_deposit(user : Pubkey) -> SwapAndAccountMetas;

/// Withdraw hyUSD from stability pool.
// NOTE: Creates Jupiter-compatible SwapAndAccountMetas for withdrawing sHYUSD from the stability pool.
pub fn stability_pool_withdraw(user : Pubkey) -> SwapAndAccountMetas;


---

# crate::jupiter
<!-- file: hylo-jupiter/src/jupiter.rs -->

## Types

/// Bidirectional single-pair Jupiter AMM client.
// NOTE: Bidirectional Jupiter AMM adapter for a single Hylo token pair, holding cached protocol state.
pub struct HyloJupiterPair< IN , OUT > where IN : TokenMint , OUT : TokenMint , {
    clock: ClockRef,
    state: Option < ProtocolState < ClockRef > >,
    _phantom: PhantomData < (IN , OUT) >,
}


## Traits

/// Pair-specific configuration and dispatch.
// NOTE: Trait providing pair-specific configuration: program ID, label, quoting, and account meta building.
pub trait PairConfig< IN : TokenMint , OUT : TokenMint > {
    fn program_id() -> Pubkey;
    fn label() -> & 'static str;
    fn key() -> Pubkey;
    fn quote(state : & ProtocolState < ClockRef >, amount : u64, input_mint : Pubkey, output_mint : Pubkey) -> Result < Quote >;
    fn build_account_metas(user : Pubkey, input_mint : Pubkey, output_mint : Pubkey) -> Result < SwapAndAccountMetas >;
}


## Impl Clone for HyloJupiterPair < IN , OUT >

// NOTE: Clone impl for HyloJupiterPair, required by Jupiter's clone_amm interface.
impl < IN : TokenMint , OUT : TokenMint >Clone for HyloJupiterPair < IN , OUT > {
    fn clone(& self) -> Self;
}


## Impl PairConfig < JITOSOL , HYUSD > for HyloJupiterPair < JITOSOL , HYUSD >

// NOTE: Jupiter pair config for JITOSOL/hyUSD mint and redeem routes.
impl PairConfig < JITOSOL , HYUSD > for HyloJupiterPair < JITOSOL , HYUSD > {
    fn program_id() -> Pubkey;
    fn label() -> & 'static str;
    fn key() -> Pubkey;
    fn quote(state : & ProtocolState < ClockRef >, amount : u64, input_mint : Pubkey, output_mint : Pubkey) -> Result < Quote >;
    fn build_account_metas(user : Pubkey, input_mint : Pubkey, output_mint : Pubkey) -> Result < SwapAndAccountMetas >;
}


## Impl PairConfig < HYLOSOL , HYUSD > for HyloJupiterPair < HYLOSOL , HYUSD >

// NOTE: Jupiter pair config for HYLOSOL/hyUSD mint and redeem routes.
impl PairConfig < HYLOSOL , HYUSD > for HyloJupiterPair < HYLOSOL , HYUSD > {
    fn program_id() -> Pubkey;
    fn label() -> & 'static str;
    fn key() -> Pubkey;
    fn quote(state : & ProtocolState < ClockRef >, amount : u64, input_mint : Pubkey, output_mint : Pubkey) -> Result < Quote >;
    fn build_account_metas(user : Pubkey, input_mint : Pubkey, output_mint : Pubkey) -> Result < SwapAndAccountMetas >;
}


## Impl PairConfig < JITOSOL , XSOL > for HyloJupiterPair < JITOSOL , XSOL >

// NOTE: Jupiter pair config for JITOSOL/xSOL mint and redeem routes.
impl PairConfig < JITOSOL , XSOL > for HyloJupiterPair < JITOSOL , XSOL > {
    fn program_id() -> Pubkey;
    fn label() -> & 'static str;
    fn key() -> Pubkey;
    fn quote(state : & ProtocolState < ClockRef >, amount : u64, input_mint : Pubkey, output_mint : Pubkey) -> Result < Quote >;
    fn build_account_metas(user : Pubkey, input_mint : Pubkey, output_mint : Pubkey) -> Result < SwapAndAccountMetas >;
}


## Impl PairConfig < HYLOSOL , XSOL > for HyloJupiterPair < HYLOSOL , XSOL >

// NOTE: Jupiter pair config for HYLOSOL/xSOL mint and redeem routes.
impl PairConfig < HYLOSOL , XSOL > for HyloJupiterPair < HYLOSOL , XSOL > {
    fn program_id() -> Pubkey;
    fn label() -> & 'static str;
    fn key() -> Pubkey;
    fn quote(state : & ProtocolState < ClockRef >, amount : u64, input_mint : Pubkey, output_mint : Pubkey) -> Result < Quote >;
    fn build_account_metas(user : Pubkey, input_mint : Pubkey, output_mint : Pubkey) -> Result < SwapAndAccountMetas >;
}


## Impl PairConfig < HYUSD , XSOL > for HyloJupiterPair < HYUSD , XSOL >

// NOTE: Jupiter pair config for hyUSD/xSOL swap routes.
impl PairConfig < HYUSD , XSOL > for HyloJupiterPair < HYUSD , XSOL > {
    fn program_id() -> Pubkey;
    fn label() -> & 'static str;
    fn key() -> Pubkey;
    fn quote(state : & ProtocolState < ClockRef >, amount : u64, input_mint : Pubkey, output_mint : Pubkey) -> Result < Quote >;
    fn build_account_metas(user : Pubkey, input_mint : Pubkey, output_mint : Pubkey) -> Result < SwapAndAccountMetas >;
}


## Impl PairConfig < HYUSD , SHYUSD > for HyloJupiterPair < HYUSD , SHYUSD >

// NOTE: Jupiter pair config for hyUSD/sHYUSD stability pool deposit and withdrawal.
impl PairConfig < HYUSD , SHYUSD > for HyloJupiterPair < HYUSD , SHYUSD > {
    fn program_id() -> Pubkey;
    fn label() -> & 'static str;
    fn key() -> Pubkey;
    fn quote(state : & ProtocolState < ClockRef >, amount : u64, input_mint : Pubkey, output_mint : Pubkey) -> Result < Quote >;
    fn build_account_metas(user : Pubkey, input_mint : Pubkey, output_mint : Pubkey) -> Result < SwapAndAccountMetas >;
}


## Impl Amm for HyloJupiterPair < IN , OUT >

// NOTE: Implements Jupiter's Amm trait enabling Hylo pairs to be discovered and routed by the Jupiter aggregator.
impl < IN , OUT >Amm for HyloJupiterPair < IN , OUT > where IN : TokenMint + 'static , OUT : TokenMint + 'static , Self : PairConfig < IN , OUT > + Clone + Send + Sync , {
    fn from_keyed_account(_keyed_account : & KeyedAccount, amm_context : & AmmContext) -> Result < Self >;
    fn label(& self) -> String;
    fn program_id(& self) -> Pubkey;
    fn key(& self) -> Pubkey;
    fn get_reserve_mints(& self) -> Vec < Pubkey >;
    fn get_accounts_to_update(& self) -> Vec < Pubkey >;
    fn update(& mut self, account_map : & AccountMap) -> Result < () >;
    fn quote(& self, params : & QuoteParams) -> Result < Quote >;
    fn get_swap_and_account_metas(& self, p : & SwapParams) -> Result < SwapAndAccountMetas >;
    fn clone_amm(& self) -> Box < dyn Amm + Send + Sync >;
}


---

# crate::util
<!-- file: hylo-jupiter/src/util.rs -->

## Functions

/// Computes fee percentage as `Decimal`.
/// 
/// # Errors
/// * Conversions
/// * Arithmetic
// NOTE: Converts extracted fee and base amounts into a Decimal percentage for Jupiter quotes.
pub fn fee_pct_decimal< Exp >(fees_extracted : UFix64 < Exp >, fee_base : UFix64 < Exp >) -> Result < Decimal >;

/// Converts [`OperationOutput`] to Jupiter [`Quote`].
/// 
/// # Errors
/// * Fee decimal conversion
// NOTE: Converts a typed OperationOutput into a Jupiter Quote with fee percentages.
pub fn operation_to_quote< InExp , OutExp , FeeExp >(op : OperationOutput < InExp , OutExp , FeeExp >) -> Result < Quote > where InExp : Integer , OutExp : Integer , FeeExp : Integer ,;

/// Generic Jupiter quote for any `IN -> OUT` pair.
/// 
/// # Errors
/// * Quote math
/// * Fee decimal conversion
// NOTE: Generic Jupiter quote function for any token pair implementing TokenOperation.
pub fn quote< IN , OUT >(state : & ProtocolState < ClockRef >, amount : u64) -> Result < Quote > where IN : TokenMint , OUT : TokenMint , ProtocolState < ClockRef > : TokenOperation < IN , OUT > , < ProtocolState < ClockRef > as TokenOperation < IN , OUT > > :: FeeExp : Integer ,;

/// Finds and deserializes an account in Jupiter's `AccountMap`.
/// 
/// # Errors
/// * Account not found in map
/// * Deserialization to `A` fails
// NOTE: Finds and deserializes a Solana account from Jupiter's AccountMap by Pubkey.
pub fn account_map_get< A : AccountDeserialize >(account_map : & AccountMap, key : & Pubkey) -> Result < A >;

/// Calls RPC to load given accounts into a map.
/// 
/// # Errors
/// * RPC fails
/// * One of the accounts is missing
// NOTE: Fetches multiple Solana accounts via RPC and loads them into Jupiter's AccountMap.
pub async fn load_account_map(client : & RpcClient, pubkeys : & [Pubkey]) -> Result < AccountMap >;

/// Loads Solana clock information from RPC.
/// 
/// # Errors
/// * RPC fails
/// * Deserialization fails
// NOTE: Fetches Solana Clock from RPC and constructs Jupiter's AmmContext.
pub async fn load_amm_context(client : & RpcClient) -> Result < AmmContext >;

/// Validates Jupiter swap parameters for Hylo compatibility.
/// 
/// # Errors
/// * `ExactOut` mode
/// * Dynamic accounts
// NOTE: Validates Jupiter swap parameters: rejects ExactOut mode and dynamic accounts.
pub fn validate_swap_params< 'a >(params : & 'a SwapParams < 'a , 'a >) -> Result < & 'a SwapParams < 'a , 'a > >;


---

# Crate: hylo_quotes (lib)

# crate
<!-- file: hylo-quotes/src/lib.rs -->

## Types

/// Typed executable quote with amounts, instructions, and compute units.
// NOTE: Typed quote containing amounts, fees, instructions, and compute units ready for transaction execution.
pub struct ExecutableQuote< In : Integer , Out : Integer , Fee : Integer > {
    pub amount_in: UFix64 < In >,
    pub amount_out: UFix64 < Out >,
    pub compute_units: u64,
    pub compute_unit_strategy: ComputeUnitStrategy,
    pub fee_amount: UFix64 < Fee >,
    pub fee_mint: Pubkey,
    pub instructions: Vec < Instruction >,
    pub address_lookup_tables: Vec < Pubkey >,
}

/// Executable quote with runtime exponent information.
// NOTE: Type-erased version of ExecutableQuote using runtime UFixValue64 for dynamic dispatch across token pairs.
pub struct ExecutableQuoteValue {
    pub amount_in: UFixValue64,
    pub amount_out: UFixValue64,
    pub compute_units: u64,
    pub compute_unit_strategy: ComputeUnitStrategy,
    pub fee_amount: UFixValue64,
    pub fee_mint: Pubkey,
    pub instructions: Vec < Instruction >,
    pub address_lookup_tables: Vec < Pubkey >,
}

// NOTE: Enum indicating whether compute units were estimated from defaults or measured via simulation.
pub enum ComputeUnitStrategy {
    Estimated,
    Simulated,
}


## Traits

/// This crate builds on [`hylo_clients::util::LST`] in core traits like
/// [`QuoteStrategy<L, OUT>`].
/// 
/// The [`Local`] marker allows us to use [`LST`] in trait bound position while
/// telling the compiler that changes in `hylo-clients` won't affect local
/// impls.
// NOTE: Orphan-rule workaround marker trait allowing hylo-quotes to impl traits bounded on LST from hylo-clients.
pub(crate) trait Local {

}


## Impl From < ExecutableQuote < In , Out , Fee > > for ExecutableQuoteValue

impl < In : Integer , Out : Integer , Fee : Integer >From < ExecutableQuote < In , Out , Fee > > for ExecutableQuoteValue {
    fn from(quote : ExecutableQuote < In , Out , Fee >) -> ExecutableQuoteValue;
}


## Impl Local for JITOSOL

// NOTE: Marks JITOSOL as a local type for trait impl coherence in hylo-quotes.
impl Local for JITOSOL {

}


## Impl Local for HYLOSOL

// NOTE: Marks HYLOSOL as a local type for trait impl coherence in hylo-quotes.
impl Local for HYLOSOL {

}


## Constants

/// Default buffered compute units for all exchange operations.
/// 
/// This is a buffered estimate (higher than measured values ~74k-97k CU) that
/// provides a safe default for all current quote operations. Measured values
/// came from calibration tool, but this value includes additional buffer for
/// safety across all operation types.
/// 
/// In the future, this could be replaced with per-instruction defaults based
/// on more comprehensive statistical analysis.
// NOTE: Default buffered compute unit budget (higher than measured ~74k-97k) used when simulation CUs are unavailable.
pub const DEFAULT_CUS_WITH_BUFFER: u64;

// NOTE: Triple-sized compute unit budget for multi-instruction operations like withdraw-and-redeem.
pub const DEFAULT_CUS_WITH_BUFFER_X3: u64;


## Re-exports

// NOTE: Marker trait for liquid staking tokens with N9 decimal precision.
pub use hylo_clients :: util :: LST;

// NOTE: Quote strategy that computes quotes from cached ProtocolState using pure math, without RPC simulation.
pub use protocol_state_strategy :: ProtocolStateStrategy;

// NOTE: Re-exports Operation and QuoteMetadata.
pub use quote_metadata :: { Operation , QuoteMetadata };

// NOTE: Core async trait: given amount, user, and slippage, produces a typed ExecutableQuote for a token pair.
pub use quote_strategy :: QuoteStrategy;

// NOTE: Trait for runtime dispatch from Pubkey mint pairs to typed QuoteStrategy implementations.
pub use runtime_quote_strategy :: RuntimeQuoteStrategy;

// NOTE: Holds compute unit count and whether it was estimated or measured from simulation.
pub use simulated_operation :: ComputeUnitInfo;

// NOTE: Quote strategy that validates quotes via RPC transaction simulation, holding both program clients.
pub use simulation_strategy :: SimulationStrategy;


---

# crate::prelude
<!-- file: hylo-quotes/src/prelude.rs -->

## Re-exports

// NOTE: Re-export of Solana RPC commitment level configuration.
pub use anchor_client :: solana_sdk :: commitment_config :: CommitmentConfig;

// NOTE: Re-export of Solana cluster configuration (Mainnet, Devnet, custom).
pub use anchor_client :: Cluster;

// NOTE: Re-export of Solana public key type.
pub use anchor_lang :: prelude :: Pubkey;

// NOTE: Re-export of anyhow::Result for ergonomic error handling.
pub use anyhow :: Result;

// NOTE: Glob re-export of hylo-fix fixed-point math prelude (UFix64, IFix64, exponents).
pub use fix :: prelude :: *;

// NOTE: Re-export of token types and TokenMint trait from hylo-idl.
pub use hylo_idl :: tokens :: { TokenMint , CBBTC , HYLOSOL , HYUSD , JITOSOL , SHYUSD , USDC , XBTC , XSOL , };

// NOTE: Re-export of protocol state types (ProtocolAccounts, ProtocolState, providers).
pub use crate :: protocol_state :: { ProtocolAccounts , ProtocolState , RpcStateProvider , StateProvider , };

// NOTE: Re-export of SimulatedOperation trait and extension.
pub use crate :: simulated_operation :: { SimulatedOperation , SimulatedOperationExt , };

// NOTE: Re-export of TokenOperation trait, extension, and output types.
pub use crate :: token_operation :: { LstSwapOperationOutput , MintOperationOutput , OperationOutput , RedeemOperationOutput , SwapOperationOutput , TokenOperation , TokenOperationExt , };

// NOTE: Re-export of the cached-state quoting strategy.
pub use crate :: ProtocolStateStrategy;

// NOTE: Re-export of the core QuoteStrategy trait.
pub use crate :: QuoteStrategy;

// NOTE: Re-export of the LST marker trait.
pub use crate :: LST;

// NOTE: Re-export of top-level quote types, compute unit types, and constants.
pub use crate :: { ComputeUnitInfo , ComputeUnitStrategy , ExecutableQuote , ExecutableQuoteValue , Operation , QuoteMetadata , DEFAULT_CUS_WITH_BUFFER , };

// NOTE: Re-export of top-level quote types, compute unit types, and constants.
pub use crate :: { RuntimeQuoteStrategy , SimulationStrategy };


---

# crate::protocol_state
<!-- file: hylo-quotes/src/protocol_state/mod.rs -->

## Re-exports

// NOTE: Typed collection of all on-chain accounts needed to construct protocol state.
pub use accounts :: ProtocolAccounts;

// NOTE: Re-exports of RpcStateProvider and StateProvider.
pub use provider :: { RpcStateProvider , StateProvider };

pub use state :: { ProtocolState , UsdcExchangeState };


---

# crate::protocol_state::accounts
<!-- file: hylo-quotes/src/protocol_state/accounts.rs -->

## Types

/// Type-safe collection of protocol state accounts
// NOTE: Typed collection of all on-chain accounts needed to construct protocol state.
pub struct ProtocolAccounts {
    pub hylo: Account,
    pub jitosol_header: Account,
    pub hylosol_header: Account,
    pub hyusd_mint: Account,
    pub shyusd_mint: Account,
    pub xsol_mint: Account,
    pub pool_config: Account,
    pub hyusd_pool: Account,
    pub xsol_pool: Account,
    pub sol_usd_pyth: Account,
    pub clock: Account,
    pub cbbtc_exo_pair: Account,
    pub cbbtc_vault: Account,
    pub xbtc_mint: Account,
    pub btc_usd_pyth: Account,
    pub usdc_pair: Account,
    pub usdc_usd_pyth: Account,
    pub jitosol_pool_state: Account,
    pub hylosol_pool_state: Account,
}


## Impl ProtocolAccounts

// NOTE: Typed collection of all on-chain accounts needed to construct protocol state.
impl ProtocolAccounts {
    pub fn pubkeys() -> Vec < Pubkey >;
    pub fn expected_count() -> usize;
    pub fn validate(pubkeys : & [Pubkey], accounts : & [Option < Account >]) -> Result < () >;
}


## Impl TryFrom < (& [Pubkey] , & [Option < Account >]) > for ProtocolAccounts

/// Convert from RPC response (pubkeys and accounts) to `ProtocolAccounts`
/// 
/// Validates that:
/// - The number of pubkeys matches the number of accounts
/// - The pubkeys match the expected protocol accounts in order
/// - All accounts are present (not None)
// NOTE: Validates and converts RPC multi-account response into typed ProtocolAccounts.
impl TryFrom < (& [Pubkey] , & [Option < Account >]) > for ProtocolAccounts {
    type Error = anyhow :: Error;
    fn try_from((pubkeys , accounts) : (& [Pubkey] , & [Option < Account >])) -> Result < Self >;
}


---

# crate::protocol_state::provider
<!-- file: hylo-quotes/src/protocol_state/provider.rs -->

## Types

/// State provider that fetches protocol state via Solana RPC
// NOTE: StateProvider implementation that fetches protocol state via Solana RPC getMultipleAccounts.
pub struct RpcStateProvider {
    rpc_client: Arc < RpcClient >,
}


## Traits

/// Trait for fetching protocol state from a data source
// NOTE: Async trait for fetching a complete ProtocolState snapshot from any data source.
pub trait StateProvider< C : SolanaClock >: Send + Sync {
    async fn fetch_state(& self) -> Result < ProtocolState < C > >;
}


## Impl StateProvider < C > for std :: sync :: Arc < T >

// NOTE: Blanket impl allowing Arc-wrapped StateProviders to be used directly.
impl < T : StateProvider < C > , C : SolanaClock >StateProvider < C > for std :: sync :: Arc < T > {
    async fn fetch_state(& self) -> Result < ProtocolState < C > >;
}


## Impl RpcStateProvider

// NOTE: StateProvider implementation that fetches protocol state via Solana RPC getMultipleAccounts.
impl RpcStateProvider {
    pub fn new(rpc_client : Arc < RpcClient >) -> Self;
}


## Impl StateProvider < Clock > for RpcStateProvider

// NOTE: Fetches all protocol accounts via RPC and deserializes into ProtocolState<Clock>.
impl StateProvider < Clock > for RpcStateProvider {
    async fn fetch_state(& self) -> Result < ProtocolState < Clock > >;
}


---

# crate::protocol_state::state
<!-- file: hylo-quotes/src/protocol_state/state.rs -->

## Types

/// USDC exchange state for stablecoin mint/redeem.
pub struct UsdcExchangeState {
    pub usdc_usd_price: hylo_core :: pyth :: PriceRange < N9 >,
    pub swap_fee: UFix64 < N9 >,
}

/// Complete snapshot of Hylo protocol state
// NOTE: Complete snapshot of Hylo protocol state including exchange context, mints, pools, and LST headers.
pub struct ProtocolState< C : SolanaClock > {
    pub exchange_context: LstExchangeContext < C >,
    pub jitosol_header: LstHeader,
    pub hylosol_header: LstHeader,
    pub hyusd_mint: Mint,
    pub xsol_mint: Mint,
    pub shyusd_mint: Mint,
    pub pool_config: PoolConfig,
    pub hyusd_pool: TokenAccount,
    pub xsol_pool: TokenAccount,
    pub fetched_at: UnixTimestamp,
    pub lst_swap_config: AssetSwapConfig,
    pub cbbtc_exchange_context: Arc < ExoExchangeContext < C > >,
    pub usdc_exchange_state: UsdcExchangeState,
    pub jitosol_stake_pool: SplStakePool,
    pub hylosol_stake_pool: SplStakePool,
}


## Functions

/// Builds the cbBTC `ExoExchangeContext` from protocol accounts.
/// 
/// # Errors
/// * Deserialization or context-load failure
fn build_cbbtc_exchange_context(clock : Clock, accounts : & ProtocolAccounts) -> Result < ExoExchangeContext < Clock > >;

/// Builds USDC exchange state from protocol accounts.
/// 
/// # Errors
/// * Deserialization or oracle failure
fn build_usdc_exchange_state(clock : & Clock, accounts : & ProtocolAccounts) -> Result < UsdcExchangeState >;


## Impl UsdcExchangeState

impl UsdcExchangeState {
    pub fn conversion(& self) -> UsdcStablecoinConversion;
    pub fn apply_fee< Exp >(& self, amount : UFix64 < Exp >) -> Result < FeeExtract < Exp > >;
}


## Impl ProtocolState < C >

// NOTE: Builder and accessor methods for constructing ProtocolState and querying LST headers.
impl < C : SolanaClock >ProtocolState < C > {
    pub fn build(clock : C, hylo : & Hylo, jitosol_header : LstHeader, hylosol_header : LstHeader, hyusd_mint : Mint, xsol_mint : Mint, shyusd_mint : Mint, pool_config : PoolConfig, hyusd_pool : TokenAccount, xsol_pool : TokenAccount, sol_usd : & PriceUpdateV2, cbbtc_exchange_context : Arc < ExoExchangeContext < C > >, usdc_exchange_state : UsdcExchangeState, jitosol_stake_pool : SplStakePool, hylosol_stake_pool : SplStakePool) -> Result < Self >;
    pub fn lst_header< L : LST >(& self) -> Result < & LstHeader >;
    pub fn stake_pool< L : LST >(& self) -> Result < & SplStakePool >;
    pub fn cbbtc_exchange_context(& self) -> & ExoExchangeContext < C >;
    pub fn usdc_exchange_state(& self) -> & UsdcExchangeState;
}


## Impl TryFrom < & ProtocolAccounts > for ProtocolState < Clock >

// NOTE: Deserializes raw ProtocolAccounts into a fully-constructed ProtocolState<Clock>.
impl TryFrom < & ProtocolAccounts > for ProtocolState < Clock > {
    type Error = anyhow :: Error;
    fn try_from(accounts : & ProtocolAccounts) -> Result < Self >;
}


---

# crate::protocol_state_strategy
<!-- file: hylo-quotes/src/protocol_state_strategy/mod.rs -->

## Types

// NOTE: Quote strategy that computes quotes from cached ProtocolState using pure math, without RPC simulation.
pub struct ProtocolStateStrategy< S > {
    pub state_provider: S,
}


## Impl ProtocolStateStrategy < S >

// NOTE: Constructor taking any StateProvider for fetching state on demand.
impl < S >ProtocolStateStrategy < S > {
    pub fn new(state_provider : S) -> Self;
}


## Impl RuntimeQuoteStrategy < C > for ProtocolStateStrategy < S >

// NOTE: Implements runtime dispatch from Pubkey pairs to typed QuoteStrategy impls on ProtocolStateStrategy.
impl < S : StateProvider < C > + Sync , C : SolanaClock >RuntimeQuoteStrategy < C > for ProtocolStateStrategy < S > {

}


---

# crate::protocol_state_strategy::router
<!-- file: hylo-quotes/src/protocol_state_strategy/router.rs -->

## Macros

macro_rules! state_quote { ... }


---

# crate::quote_metadata
<!-- file: hylo-quotes/src/quote_metadata.rs -->

## Types

/// Operation type for a quote
// NOTE: Enum of all supported quote operation types (mint, redeem, swap, deposit, withdraw, etc.).
pub enum Operation {
    MintStablecoinLst,
    RedeemStablecoinLst,
    MintLevercoinLst,
    RedeemLevercoinLst,
    ConvertStableToLeverLst,
    ConvertLeverToStableLst,
    SwapLstToLst,
    DepositToStabilityPool,
    WithdrawFromStabilityPool,
    MintStablecoinUsdc,
    RedeemStablecoinUsdc,
    MintStablecoinExo,
    RedeemStablecoinExo,
    MintLevercoinExo,
    RedeemLevercoinExo,
    ConvertStableToLeverExo,
    ConvertLeverToStableExo,
    SwapLstToUsdc,
    SwapUsdcToLst,
    SwapExoToUsdc,
    SwapUsdcToExo,
}

/// Metadata for a quote route.
// NOTE: Pairs an Operation enum with a human-readable description of the quote route.
pub struct QuoteMetadata {
    pub operation: Operation,
    pub description: String,
}


## Impl Operation

// NOTE: Enum of all supported quote operation types (mint, redeem, swap, deposit, withdraw, etc.).
impl Operation {
    pub fn as_str(& self) -> & 'static str;
}


## Impl AsRef < str > for Operation

// NOTE: Returns the string name of a quote Operation variant.
impl AsRef < str > for Operation {
    fn as_ref(& self) -> & str;
}


## Impl std :: fmt :: Display for Operation

// NOTE: Display formatting for Operation using its string name.
impl std :: fmt :: Display for Operation {
    fn fmt(& self, f : & mut std :: fmt :: Formatter < '_ >) -> std :: fmt :: Result;
}


## Impl QuoteMetadata

// NOTE: Pairs an Operation enum with a human-readable description of the quote route.
impl QuoteMetadata {
    pub fn new(operation : Operation, description : impl Into < String >) -> Self;
}


---

# crate::quote_strategy
<!-- file: hylo-quotes/src/quote_strategy.rs -->

## Traits

/// Trait for strategies that compute quotes for token pair operations.
// NOTE: Core async trait: given amount, user, and slippage, produces a typed ExecutableQuote for a token pair.
pub trait QuoteStrategy< IN : TokenMint , OUT : TokenMint , C : SolanaClock > {
    type FeeExp: Integer;
    async fn get_quote(& self, amount_in : u64, user : Pubkey, slippage_tolerance : u64) -> Result < ExecutableQuote < IN :: Exp , OUT :: Exp , Self :: FeeExp > >;
}


---

# crate::runtime_quote_strategy
<!-- file: hylo-quotes/src/runtime_quote_strategy.rs -->

## Macros

// NOTE: Macro generating match arms for all 16 supported token pair routes in RuntimeQuoteStrategy.
macro_rules! runtime_quote_strategies { ... }


---

# crate::simulated_operation
<!-- file: hylo-quotes/src/simulated_operation/mod.rs -->

## Types

/// Compute unit details from simulation.
// NOTE: Holds compute unit count and whether it was estimated or measured from simulation.
pub struct ComputeUnitInfo {
    pub compute_units: u64,
    pub strategy: ComputeUnitStrategy,
}


## Traits

/// Simulation counterpart to [`TokenOperation`]—extracts output from events
/// rather than computing from state.
/// 
/// [`TokenOperation`]: crate::token_operation::TokenOperation
// NOTE: Trait for extracting token operation output from on-chain simulation event logs.
pub trait SimulatedOperation< IN : TokenMint , OUT : TokenMint > {
    type FeeExp: Integer;
    type Event: AnchorDeserialize;
    fn extract_output(event : & Self :: Event) -> Result < OperationOutput < IN :: Exp , OUT :: Exp , Self :: FeeExp > >;
}

/// Turbofish helper for [`SimulatedOperation`].
// NOTE: Turbofish helper trait combining simulation execution with output extraction.
pub trait SimulatedOperationExt {
    fn extract_output< IN : TokenMint , OUT : TokenMint >(event : & < Self as SimulatedOperation < IN , OUT > > :: Event) -> Result < OperationOutput < IN :: Exp , OUT :: Exp , < Self as SimulatedOperation < IN , OUT > > :: FeeExp , > , >;
    async fn simulate_output< IN : TokenMint , OUT : TokenMint >(& self, user : Pubkey, inputs : < Self as BuildTransactionData < IN , OUT > > :: Inputs) -> Result < (OperationOutput < IN :: Exp , OUT :: Exp , < Self as SimulatedOperation < IN , OUT > > :: FeeExp , > , ComputeUnitInfo ,) >;
}


## Impl Default for ComputeUnitInfo

// NOTE: Defaults to the buffered estimate compute units with Estimated strategy.
impl Default for ComputeUnitInfo {
    fn default() -> Self;
}


## Impl ComputeUnitInfo

// NOTE: Holds compute unit count and whether it was estimated or measured from simulation.
impl ComputeUnitInfo {
    pub fn from_simulation(cus : Option < u64 >) -> Self;
}


## Impl SimulatedOperationExt for X

// NOTE: Blanket impl providing simulate_output which builds, simulates, and extracts results.
impl < X >SimulatedOperationExt for X {
    fn extract_output< IN : TokenMint , OUT : TokenMint >(event : & < Self as SimulatedOperation < IN , OUT > > :: Event) -> Result < OperationOutput < IN :: Exp , OUT :: Exp , < Self as SimulatedOperation < IN , OUT > > :: FeeExp , > , >;
    async fn simulate_output< IN : TokenMint , OUT : TokenMint >(& self, user : Pubkey, inputs : < Self as BuildTransactionData < IN , OUT > > :: Inputs) -> Result < (OperationOutput < IN :: Exp , OUT :: Exp , < Self as SimulatedOperation < IN , OUT > > :: FeeExp , > , ComputeUnitInfo ,) >;
}


---

# crate::simulated_operation::exchange
<!-- file: hylo-quotes/src/simulated_operation/exchange.rs -->

## Impl SimulatedOperation < L , HYUSD > for RouterClient

/// Mint stablecoin from LST.
impl < L : LST + Local >SimulatedOperation < L , HYUSD > for RouterClient {
    type FeeExp = N9;
    type Event = MintStablecoinLstEvent;
    fn extract_output(event : & MintStablecoinLstEvent) -> Result < MintOperationOutput >;
}


## Impl SimulatedOperation < HYUSD , L > for RouterClient

/// Redeem stablecoin for LST.
impl < L : LST + Local >SimulatedOperation < HYUSD , L > for RouterClient {
    type FeeExp = N9;
    type Event = RedeemStablecoinLstEvent;
    fn extract_output(event : & RedeemStablecoinLstEvent) -> Result < RedeemOperationOutput >;
}


## Impl SimulatedOperation < L , XSOL > for RouterClient

/// Mint levercoin from LST.
impl < L : LST + Local >SimulatedOperation < L , XSOL > for RouterClient {
    type FeeExp = N9;
    type Event = MintLevercoinLstEvent;
    fn extract_output(event : & MintLevercoinLstEvent) -> Result < MintOperationOutput >;
}


## Impl SimulatedOperation < XSOL , L > for RouterClient

/// Redeem levercoin for LST.
impl < L : LST + Local >SimulatedOperation < XSOL , L > for RouterClient {
    type FeeExp = N9;
    type Event = RedeemLevercoinLstEvent;
    fn extract_output(event : & RedeemLevercoinLstEvent) -> Result < RedeemOperationOutput >;
}


## Impl SimulatedOperation < HYUSD , XSOL > for RouterClient

/// Convert stablecoin to levercoin.
impl SimulatedOperation < HYUSD , XSOL > for RouterClient {
    type FeeExp = N6;
    type Event = ConvertStableToLeverLstEvent;
    fn extract_output(event : & ConvertStableToLeverLstEvent) -> Result < SwapOperationOutput >;
}


## Impl SimulatedOperation < XSOL , HYUSD > for RouterClient

/// Convert levercoin to stablecoin.
impl SimulatedOperation < XSOL , HYUSD > for RouterClient {
    type FeeExp = N6;
    type Event = ConvertLeverToStableLstEvent;
    fn extract_output(event : & ConvertLeverToStableLstEvent) -> Result < SwapOperationOutput >;
}


## Impl SimulatedOperation < L1 , L2 > for RouterClient

/// Swap between LSTs.
impl < L1 : LST + Local , L2 : LST + Local >SimulatedOperation < L1 , L2 > for RouterClient {
    type FeeExp = N9;
    type Event = SwapLstToLstEvent;
    fn extract_output(event : & SwapLstToLstEvent) -> Result < LstSwapOperationOutput >;
}


## Impl SimulatedOperation < USDC , HYUSD > for RouterClient

/// Mint stablecoin from USDC.
/// 
/// On-chain: USDC is normalized to N9 before fee extraction, so
/// event amounts `usdc_deposited` and `usdc_fees` are N9.
impl SimulatedOperation < USDC , HYUSD > for RouterClient {
    type FeeExp = N9;
    type Event = MintStablecoinUsdcEvent;
    fn extract_output(event : & MintStablecoinUsdcEvent) -> Result < OperationOutput < N6 , N6 , N9 > >;
}


## Impl SimulatedOperation < HYUSD , USDC > for RouterClient

/// Redeem stablecoin to USDC.
/// 
/// On-chain: fee is applied to HYUSD input before conversion.
/// `fee_base` is the total HYUSD input (`stablecoin_burned +
/// stablecoin_fees`) and `fee_mint` is HYUSD.
impl SimulatedOperation < HYUSD , USDC > for RouterClient {
    type FeeExp = N6;
    type Event = RedeemStablecoinUsdcEvent;
    fn extract_output(event : & RedeemStablecoinUsdcEvent) -> Result < SwapOperationOutput >;
}


## Impl SimulatedOperation < CBBTC , HYUSD > for RouterClient

/// Mint stablecoin from exo collateral (cbBTC -> HYUSD).
impl SimulatedOperation < CBBTC , HYUSD > for RouterClient {
    type FeeExp = N9;
    type Event = MintStablecoinExoEvent;
    fn extract_output(event : & MintStablecoinExoEvent) -> Result < OperationOutput < N8 , N6 , N9 > >;
}


## Impl SimulatedOperation < HYUSD , CBBTC > for RouterClient

/// Redeem stablecoin for exo collateral (HYUSD -> cbBTC).
impl SimulatedOperation < HYUSD , CBBTC > for RouterClient {
    type FeeExp = N9;
    type Event = RedeemStablecoinExoEvent;
    fn extract_output(event : & RedeemStablecoinExoEvent) -> Result < OperationOutput < N6 , N8 , N9 > >;
}


## Impl SimulatedOperation < CBBTC , XBTC > for RouterClient

/// Mint levercoin from exo collateral (cbBTC -> xBTC).
impl SimulatedOperation < CBBTC , XBTC > for RouterClient {
    type FeeExp = N9;
    type Event = MintLevercoinExoEvent;
    fn extract_output(event : & MintLevercoinExoEvent) -> Result < OperationOutput < N8 , N6 , N9 > >;
}


## Impl SimulatedOperation < XBTC , CBBTC > for RouterClient

/// Redeem levercoin for exo collateral (xBTC -> cbBTC).
impl SimulatedOperation < XBTC , CBBTC > for RouterClient {
    type FeeExp = N9;
    type Event = RedeemLevercoinExoEvent;
    fn extract_output(event : & RedeemLevercoinExoEvent) -> Result < OperationOutput < N6 , N8 , N9 > >;
}


## Impl SimulatedOperation < HYUSD , XBTC > for RouterClient

/// Convert stablecoin to exo levercoin (HYUSD -> xBTC).
impl SimulatedOperation < HYUSD , XBTC > for RouterClient {
    type FeeExp = N6;
    type Event = ConvertStableToLeverExoEvent;
    fn extract_output(event : & ConvertStableToLeverExoEvent) -> Result < SwapOperationOutput >;
}


## Impl SimulatedOperation < L , USDC > for RouterClient

/// Swap LST for USDC.
impl < L : LST + Local >SimulatedOperation < L , USDC > for RouterClient {
    type FeeExp = N9;
    type Event = SwapLstToUsdcEvent;
    fn extract_output(event : & SwapLstToUsdcEvent) -> Result < OperationOutput < N9 , N6 , N9 > >;
}


## Impl SimulatedOperation < USDC , L > for RouterClient

/// Swap USDC for LST.
impl < L : LST + Local >SimulatedOperation < USDC , L > for RouterClient {
    type FeeExp = N6;
    type Event = SwapUsdcToLstEvent;
    fn extract_output(event : & SwapUsdcToLstEvent) -> Result < OperationOutput < N6 , N9 , N6 > >;
}


## Impl SimulatedOperation < CBBTC , USDC > for RouterClient

/// Swap cbBTC for USDC.
impl SimulatedOperation < CBBTC , USDC > for RouterClient {
    type FeeExp = N8;
    type Event = SwapExoToUsdcEvent;
    fn extract_output(event : & SwapExoToUsdcEvent) -> Result < OperationOutput < N8 , N6 , N8 > >;
}


## Impl SimulatedOperation < USDC , CBBTC > for RouterClient

/// Swap USDC for cbBTC.
impl SimulatedOperation < USDC , CBBTC > for RouterClient {
    type FeeExp = N6;
    type Event = SwapUsdcToExoEvent;
    fn extract_output(event : & SwapUsdcToExoEvent) -> Result < OperationOutput < N6 , N8 , N6 > >;
}


## Impl SimulatedOperation < XBTC , HYUSD > for RouterClient

/// Convert xBTC to hyUSD.
impl SimulatedOperation < XBTC , HYUSD > for RouterClient {
    type FeeExp = N6;
    type Event = ConvertLeverToStableExoEvent;
    fn extract_output(event : & ConvertLeverToStableExoEvent) -> Result < SwapOperationOutput >;
}


---

# crate::simulated_operation::stability_pool
<!-- file: hylo-quotes/src/simulated_operation/stability_pool.rs -->

## Impl SimulatedOperation < HYUSD , SHYUSD > for RouterClient

/// Deposit stablecoin.
impl SimulatedOperation < HYUSD , SHYUSD > for RouterClient {
    type FeeExp = N6;
    type Event = UserDepositEvent;
    fn extract_output(event : & UserDepositEvent) -> Result < SwapOperationOutput >;
}


## Impl SimulatedOperation < SHYUSD , HYUSD > for RouterClient

/// Withdraw stablecoin.
impl SimulatedOperation < SHYUSD , HYUSD > for RouterClient {
    type FeeExp = N6;
    type Event = UserWithdrawEvent;
    fn extract_output(event : & UserWithdrawEvent) -> Result < SwapOperationOutput >;
}


---

# crate::simulation_strategy
<!-- file: hylo-quotes/src/simulation_strategy/mod.rs -->

## Types

// NOTE: Quote strategy that validates quotes via RPC transaction simulation, holding both program clients.
pub struct SimulationStrategy {
    pub router_client: RouterClient,
}


## Impl SimulationStrategy

// NOTE: Quote strategy that validates quotes via RPC transaction simulation, holding both program clients.
impl SimulationStrategy {
    pub fn new(router_client : RouterClient) -> SimulationStrategy;
}


## Impl RuntimeQuoteStrategy < Clock > for SimulationStrategy

// NOTE: Implements runtime dispatch from Pubkey pairs to typed QuoteStrategy impls on SimulationStrategy.
impl RuntimeQuoteStrategy < Clock > for SimulationStrategy {

}


## Impl TransactionSyntax for SimulationStrategy

// NOTE: Delegates transaction execution to the appropriate underlying program client.
impl TransactionSyntax for SimulationStrategy {

}


---

# crate::simulation_strategy::router
<!-- file: hylo-quotes/src/simulation_strategy/router.rs -->

## Functions

fn sim_args(amount_in : u64, user : Pubkey) -> RouterArgs;

fn quote_args< E : Integer >(amount_in : u64, user : Pubkey, out_amount : UFix64 < E >, slippage_tolerance : u64) -> RouterArgs;


## Macros

macro_rules! simulation_quote { ... }


---

# crate::token_operation
<!-- file: hylo-quotes/src/token_operation/mod.rs -->

## Types

// NOTE: Result of a token operation: input amount, output amount, fee amount, fee mint, and fee base.
pub struct OperationOutput< InExp : Integer , OutExp : Integer , FeeExp : Integer > {
    pub in_amount: UFix64 < InExp >,
    pub out_amount: UFix64 < OutExp >,
    pub fee_amount: UFix64 < FeeExp >,
    pub fee_mint: Pubkey,
    pub fee_base: UFix64 < FeeExp >,
}

// NOTE: Type alias for OperationOutput<N9, N6, N9> from mint operations.
pub type MintOperationOutput = OperationOutput < N9 , N6 , N9 >;

// NOTE: Type alias for OperationOutput<N6, N9, N9> from redeem operations.
pub type RedeemOperationOutput = OperationOutput < N6 , N9 , N9 >;

// NOTE: Type alias for OperationOutput<N6, N6, N6> from swap operations.
pub type SwapOperationOutput = OperationOutput < N6 , N6 , N6 >;

// NOTE: Type alias for OperationOutput<N9, N9, N9> from LST-to-LST swaps.
pub type LstSwapOperationOutput = OperationOutput < N9 , N9 , N9 >;


## Traits

// NOTE: Pure math trait computing token output from input amount using protocol state, without RPC.
pub trait TokenOperation< IN : TokenMint , OUT : TokenMint > {
    type FeeExp: Integer;
    fn compute_output(& self, amount_in : UFix64 < IN :: Exp >) -> Result < OperationOutput < IN :: Exp , OUT :: Exp , Self :: FeeExp > >;
}

/// Turbofish helper for [`TokenOperation`].
// NOTE: Turbofish helper: state.output::<IN, OUT>(amount) instead of TokenOperation::compute_output.
pub trait TokenOperationExt {
    fn output< IN , OUT >(& self, amount_in : UFix64 < IN :: Exp >) -> Result < OperationOutput < IN :: Exp , OUT :: Exp , < Self as TokenOperation < IN , OUT > > :: FeeExp , > , >;
}


## Impl TokenOperationExt for X

// NOTE: Blanket impl delegating to TokenOperation for any type that implements it.
impl < X >TokenOperationExt for X {
    fn output< IN , OUT >(& self, amount_in : UFix64 < IN :: Exp >) -> Result < OperationOutput < IN :: Exp , OUT :: Exp , < Self as TokenOperation < IN , OUT > > :: FeeExp , > , >;
}


---

# crate::token_operation::exchange
<!-- file: hylo-quotes/src/token_operation/exchange.rs -->

## Impl TokenOperation < L , HYUSD > for ProtocolState < C >

/// Mint stablecoin (HYUSD) from LST collateral.
// NOTE: Computes stablecoin mint: deposits LST, applies fees, converts to hyUSD output.
impl < L : LST + Local , C : SolanaClock >TokenOperation < L , HYUSD > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N9 >) -> Result < MintOperationOutput >;
}


## Impl TokenOperation < HYUSD , L > for ProtocolState < C >

/// Redeem stablecoin (HYUSD) for LST collateral.
// NOTE: Computes stablecoin redemption: burns hyUSD, applies fees, converts to LST output.
impl < L : LST + Local , C : SolanaClock >TokenOperation < HYUSD , L > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < < HYUSD as TokenMint > :: Exp >) -> Result < RedeemOperationOutput >;
}


## Impl TokenOperation < L , XSOL > for ProtocolState < C >

/// Mint levercoin (XSOL) from LST collateral.
// NOTE: Computes levercoin mint: deposits LST, applies fees, converts to xSOL output.
impl < L : LST + Local , C : SolanaClock >TokenOperation < L , XSOL > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N9 >) -> Result < MintOperationOutput >;
}


## Impl TokenOperation < XSOL , L > for ProtocolState < C >

/// Redeem levercoin (XSOL) for LST collateral.
// NOTE: Computes levercoin redemption: burns xSOL, applies fees, converts to LST output.
impl < L : LST + Local , C : SolanaClock >TokenOperation < XSOL , L > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < < XSOL as TokenMint > :: Exp >) -> Result < RedeemOperationOutput >;
}


## Impl TokenOperation < HYUSD , XSOL > for ProtocolState < C >

/// Swap stablecoin (HYUSD) to levercoin (XSOL).
// NOTE: Computes hyUSD-to-xSOL swap using swap conversion and levercoin fees.
impl < C : SolanaClock >TokenOperation < HYUSD , XSOL > for ProtocolState < C > {
    type FeeExp = < HYUSD as TokenMint > :: Exp;
    fn compute_output(& self, in_amount : UFix64 < < HYUSD as TokenMint > :: Exp >) -> Result < SwapOperationOutput >;
}


## Impl TokenOperation < XSOL , HYUSD > for ProtocolState < C >

/// Swap levercoin (XSOL) to stablecoin (HYUSD).
// NOTE: Computes xSOL-to-hyUSD swap using swap conversion and levercoin fees.
impl < C : SolanaClock >TokenOperation < XSOL , HYUSD > for ProtocolState < C > {
    type FeeExp = < HYUSD as TokenMint > :: Exp;
    fn compute_output(& self, in_amount : UFix64 < < XSOL as TokenMint > :: Exp >) -> Result < SwapOperationOutput >;
}


## Impl TokenOperation < L1 , L2 > for ProtocolState < C >

/// Swap LST -> LST.
// NOTE: Computes LST-to-LST swap using both LST headers' SOL prices and the swap fee.
impl < L1 : LST + Local , L2 : LST + Local , C : SolanaClock >TokenOperation < L1 , L2 > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N9 >) -> Result < LstSwapOperationOutput >;
}


## Impl TokenOperation < USDC , HYUSD > for ProtocolState < C >

/// Mint stablecoin (HYUSD) from USDC.
/// 
/// On-chain flow: normalize USDC to N9, apply fee at N9, then convert
/// to stablecoin. Fee is denominated in USDC (at N9 precision).
impl < C : SolanaClock >TokenOperation < USDC , HYUSD > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N6 , N9 > >;
}


## Impl TokenOperation < HYUSD , USDC > for ProtocolState < C >

/// Redeem stablecoin (HYUSD) for USDC.
/// 
/// On-chain flow: apply fee to HYUSD input first, then convert
/// remaining HYUSD to USDC. Fee is denominated in HYUSD.
impl < C : SolanaClock >TokenOperation < HYUSD , USDC > for ProtocolState < C > {
    type FeeExp = N6;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N6 , N6 > >;
}


## Impl TokenOperation < CBBTC , HYUSD > for ProtocolState < C >

/// Mint stablecoin (HYUSD) from cbBTC.
impl < C : SolanaClock >TokenOperation < CBBTC , HYUSD > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N8 >) -> Result < OperationOutput < N8 , N6 , N9 > >;
}


## Impl TokenOperation < HYUSD , CBBTC > for ProtocolState < C >

/// Redeem stablecoin (HYUSD) for cbBTC.
impl < C : SolanaClock >TokenOperation < HYUSD , CBBTC > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N8 , N9 > >;
}


## Impl TokenOperation < CBBTC , XBTC > for ProtocolState < C >

/// Mint levercoin (xBTC) from cbBTC.
impl < C : SolanaClock >TokenOperation < CBBTC , XBTC > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N8 >) -> Result < OperationOutput < N8 , N6 , N9 > >;
}


## Impl TokenOperation < XBTC , CBBTC > for ProtocolState < C >

/// Redeem levercoin (xBTC) for cbBTC.
impl < C : SolanaClock >TokenOperation < XBTC , CBBTC > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N8 , N9 > >;
}


## Impl TokenOperation < HYUSD , XBTC > for ProtocolState < C >

/// Swap stablecoin (HYUSD) to exo levercoin (xBTC).
impl < C : SolanaClock >TokenOperation < HYUSD , XBTC > for ProtocolState < C > {
    type FeeExp = N6;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N6 , N6 > >;
}


## Impl TokenOperation < XBTC , HYUSD > for ProtocolState < C >

/// Swap exo levercoin (xBTC) to stablecoin (HYUSD).
impl < C : SolanaClock >TokenOperation < XBTC , HYUSD > for ProtocolState < C > {
    type FeeExp = N6;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N6 , N6 > >;
}


## Impl TokenOperation < JITOSOL , USDC > for ProtocolState < C >

/// Swap `JitoSOL` for USDC.
impl < C : SolanaClock >TokenOperation < JITOSOL , USDC > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N9 >) -> Result < OperationOutput < N9 , N6 , N9 > >;
}


## Impl TokenOperation < HYLOSOL , USDC > for ProtocolState < C >

/// Swap `hyloSOL` for USDC.
impl < C : SolanaClock >TokenOperation < HYLOSOL , USDC > for ProtocolState < C > {
    type FeeExp = N9;
    fn compute_output(& self, in_amount : UFix64 < N9 >) -> Result < OperationOutput < N9 , N6 , N9 > >;
}


## Impl TokenOperation < USDC , JITOSOL > for ProtocolState < C >

/// Swap USDC for `JitoSOL`.
impl < C : SolanaClock >TokenOperation < USDC , JITOSOL > for ProtocolState < C > {
    type FeeExp = N6;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N9 , N6 > >;
}


## Impl TokenOperation < USDC , HYLOSOL > for ProtocolState < C >

/// Swap USDC for `hyloSOL`.
impl < C : SolanaClock >TokenOperation < USDC , HYLOSOL > for ProtocolState < C > {
    type FeeExp = N6;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N9 , N6 > >;
}


## Impl TokenOperation < CBBTC , USDC > for ProtocolState < C >

/// Swap cbBTC for USDC.
impl < C : SolanaClock >TokenOperation < CBBTC , USDC > for ProtocolState < C > {
    type FeeExp = N8;
    fn compute_output(& self, in_amount : UFix64 < N8 >) -> Result < OperationOutput < N8 , N6 , N8 > >;
}


## Impl TokenOperation < USDC , CBBTC > for ProtocolState < C >

/// Swap USDC for cbBTC.
impl < C : SolanaClock >TokenOperation < USDC , CBBTC > for ProtocolState < C > {
    type FeeExp = N6;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < OperationOutput < N6 , N8 , N6 > >;
}


---

# crate::token_operation::stability_pool
<!-- file: hylo-quotes/src/token_operation/stability_pool.rs -->

## Impl TokenOperation < HYUSD , SHYUSD > for ProtocolState < C >

/// Deposit stablecoin (HYUSD) into stability pool for LP token (SHYUSD).
// NOTE: Computes stability pool deposit: hyUSD in, sHYUSD LP tokens out based on pool NAV.
impl < C : SolanaClock >TokenOperation < HYUSD , SHYUSD > for ProtocolState < C > {
    type FeeExp = N6;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < SwapOperationOutput >;
}


## Impl TokenOperation < SHYUSD , HYUSD > for ProtocolState < C >

/// Withdraw LP token (SHYUSD) from stability pool for stablecoin (HYUSD).
// NOTE: Computes stability pool withdrawal: sHYUSD in, proportional hyUSD out minus withdrawal fee.
impl < C : SolanaClock >TokenOperation < SHYUSD , HYUSD > for ProtocolState < C > {
    type FeeExp = N6;
    fn compute_output(& self, in_amount : UFix64 < N6 >) -> Result < SwapOperationOutput >;
}


---

