## Trait Implementations

Amm                                                            <- HyloJupiterPair < IN , OUT >
AsRef < str >                                                  <- Operation
Clone                                                          <- HyloJupiterPair < IN , OUT >
Default                                                        <- ComputeUnitInfo, VirtualStablecoin
Display                                                        <- StabilityMode
ExchangeContext                                                <- ExoExchangeContext < C >, LstExchangeContext < C >
FeeController                                                  <- LevercoinFees
From < ExecutableQuote < In , Out , Fee > >                    <- ExecutableQuoteValue
From < FeePair >                                               <- hylo_idl :: exchange :: types :: FeePair
From < FundingRateConfig >                                     <- hylo_idl :: exchange :: types :: FundingRateConfig
From < LevercoinFees >                                         <- hylo_idl :: exchange :: types :: LevercoinFees
From < RebalanceCurveConfig >                                  <- hylo_idl :: exchange :: types :: RebalanceCurveConfig
From < SlippageConfig >                                        <- hylo_idl :: exchange :: types :: SlippageConfig, hylo_idl :: router :: types :: SlippageConfig
From < StablecoinFees >                                        <- hylo_idl :: exchange :: types :: StablecoinFees
From < UFixValue64 >                                           <- crate :: exchange :: types :: UFixValue64, crate :: router :: types :: UFixValue64
From < YieldHarvestConfig >                                    <- hylo_idl :: exchange :: types :: YieldHarvestConfig
From < crate :: exchange :: types :: SlippageConfig >          <- crate :: router :: types :: SlippageConfig
From < crate :: exchange :: types :: UFixValue64 >             <- UFixValue64
From < crate :: stability_pool :: types :: UFixValue64 >       <- UFixValue64
From < hylo_idl :: exchange :: types :: FeePair >              <- FeePair
From < hylo_idl :: exchange :: types :: FundingRateConfig >    <- FundingRateConfig
From < hylo_idl :: exchange :: types :: HarvestCache >         <- HarvestCache
From < hylo_idl :: exchange :: types :: LevercoinFees >        <- LevercoinFees
From < hylo_idl :: exchange :: types :: LstSolPrice >          <- LstSolPrice
From < hylo_idl :: exchange :: types :: RebalanceCurveConfig > <- RebalanceCurveConfig
From < hylo_idl :: exchange :: types :: StablecoinFees >       <- StablecoinFees
From < hylo_idl :: exchange :: types :: TotalSolCache >        <- TotalSolCache
From < hylo_idl :: exchange :: types :: VirtualStablecoin >    <- VirtualStablecoin
From < hylo_idl :: exchange :: types :: YieldHarvestConfig >   <- YieldHarvestConfig
InstructionBuilderExt                                          <- X
InterpolatedFeeController < 20 >                               <- InterpolatedRedeemFees
InterpolatedFeeController < 21 >                               <- InterpolatedMintFees
LST                                                            <- HYLOSOL, JITOSOL
Local                                                          <- HYLOSOL, JITOSOL
PairConfig < HYLOSOL , HYUSD >                                 <- HyloJupiterPair < HYLOSOL , HYUSD >
PairConfig < HYLOSOL , XSOL >                                  <- HyloJupiterPair < HYLOSOL , XSOL >
PairConfig < HYUSD , SHYUSD >                                  <- HyloJupiterPair < HYUSD , SHYUSD >
PairConfig < HYUSD , XSOL >                                    <- HyloJupiterPair < HYUSD , XSOL >
PairConfig < JITOSOL , HYUSD >                                 <- HyloJupiterPair < JITOSOL , HYUSD >
PairConfig < JITOSOL , XSOL >                                  <- HyloJupiterPair < JITOSOL , XSOL >
ProgramClient                                                  <- ExchangeClient, RouterClient, StabilityPoolClient
RebalancePriceController                                       <- BuyPriceCurve, SellPriceCurve
RuntimeQuoteStrategy < C >                                     <- ProtocolStateStrategy < S >
RuntimeQuoteStrategy < Clock >                                 <- SimulationStrategy
SimulatedOperation < CBBTC , HYUSD >                           <- RouterClient
SimulatedOperation < CBBTC , USDC >                            <- RouterClient
SimulatedOperation < CBBTC , XBTC >                            <- RouterClient
SimulatedOperation < HYUSD , CBBTC >                           <- RouterClient
SimulatedOperation < HYUSD , L >                               <- RouterClient
SimulatedOperation < HYUSD , SHYUSD >                          <- RouterClient
SimulatedOperation < HYUSD , USDC >                            <- RouterClient
SimulatedOperation < HYUSD , XBTC >                            <- RouterClient
SimulatedOperation < HYUSD , XSOL >                            <- RouterClient
SimulatedOperation < L , HYUSD >                               <- RouterClient
SimulatedOperation < L , USDC >                                <- RouterClient
SimulatedOperation < L , XSOL >                                <- RouterClient
SimulatedOperation < L1 , L2 >                                 <- RouterClient
SimulatedOperation < SHYUSD , HYUSD >                          <- RouterClient
SimulatedOperation < USDC , CBBTC >                            <- RouterClient
SimulatedOperation < USDC , HYUSD >                            <- RouterClient
SimulatedOperation < USDC , L >                                <- RouterClient
SimulatedOperation < XBTC , CBBTC >                            <- RouterClient
SimulatedOperation < XBTC , HYUSD >                            <- RouterClient
SimulatedOperation < XSOL , HYUSD >                            <- RouterClient
SimulatedOperation < XSOL , L >                                <- RouterClient
SimulatedOperationExt                                          <- X
SolanaClock                                                    <- Clock, ClockRef
StakePool                                                      <- HYLOSOL, JITOSOL
StateProvider < C >                                            <- std :: sync :: Arc < T >
StateProvider < Clock >                                        <- RpcStateProvider
TokenMint                                                      <- CBBTC, HYLOSOL, HYUSD, JITOSOL, SHYUSD, USDC, XBTC, XSOL
TokenOperation < CBBTC , HYUSD >                               <- ProtocolState < C >
TokenOperation < CBBTC , USDC >                                <- ProtocolState < C >
TokenOperation < CBBTC , XBTC >                                <- ProtocolState < C >
TokenOperation < HYLOSOL , USDC >                              <- ProtocolState < C >
TokenOperation < HYUSD , CBBTC >                               <- ProtocolState < C >
TokenOperation < HYUSD , L >                                   <- ProtocolState < C >
TokenOperation < HYUSD , SHYUSD >                              <- ProtocolState < C >
TokenOperation < HYUSD , USDC >                                <- ProtocolState < C >
TokenOperation < HYUSD , XBTC >                                <- ProtocolState < C >
TokenOperation < HYUSD , XSOL >                                <- ProtocolState < C >
TokenOperation < JITOSOL , USDC >                              <- ProtocolState < C >
TokenOperation < L , HYUSD >                                   <- ProtocolState < C >
TokenOperation < L , XSOL >                                    <- ProtocolState < C >
TokenOperation < L1 , L2 >                                     <- ProtocolState < C >
TokenOperation < SHYUSD , HYUSD >                              <- ProtocolState < C >
TokenOperation < USDC , CBBTC >                                <- ProtocolState < C >
TokenOperation < USDC , HYLOSOL >                              <- ProtocolState < C >
TokenOperation < USDC , HYUSD >                                <- ProtocolState < C >
TokenOperation < USDC , JITOSOL >                              <- ProtocolState < C >
TokenOperation < XBTC , CBBTC >                                <- ProtocolState < C >
TokenOperation < XBTC , HYUSD >                                <- ProtocolState < C >
TokenOperation < XSOL , HYUSD >                                <- ProtocolState < C >
TokenOperation < XSOL , L >                                    <- ProtocolState < C >
TokenOperationExt                                              <- X
TransactionSyntax                                              <- RouterClient, SimulationStrategy
TryFrom < & ProtocolAccounts >                                 <- ProtocolState < Clock >
TryFrom < (& [Pubkey] , & [Option < Account >]) >              <- ProtocolAccounts
TryFrom < crate :: exchange :: types :: UFixValue64 >          <- UFix64 < Exp >
TryFrom < crate :: stability_pool :: types :: UFixValue64 >    <- UFix64 < Exp >
std :: fmt :: Display                                          <- Operation

## Error Chains

ExecutableQuote < In , Out , Fee > -> ExecutableQuoteValue
SlippageConfig -> hylo_idl :: exchange :: types :: SlippageConfig
SlippageConfig -> hylo_idl :: router :: types :: SlippageConfig
crate :: exchange :: types :: SlippageConfig -> crate :: router :: types :: SlippageConfig
crate :: stability_pool :: types :: UFixValue64 -> UFixValue64 -> crate :: exchange :: types :: UFixValue64
crate :: stability_pool :: types :: UFixValue64 -> UFixValue64 -> crate :: router :: types :: UFixValue64
hylo_idl :: exchange :: types :: HarvestCache -> HarvestCache
hylo_idl :: exchange :: types :: LstSolPrice -> LstSolPrice
hylo_idl :: exchange :: types :: TotalSolCache -> TotalSolCache
hylo_idl :: exchange :: types :: VirtualStablecoin -> VirtualStablecoin
FeePair -> hylo_idl :: exchange :: types :: FeePair
FundingRateConfig -> hylo_idl :: exchange :: types :: FundingRateConfig
LevercoinFees -> hylo_idl :: exchange :: types :: LevercoinFees
RebalanceCurveConfig -> hylo_idl :: exchange :: types :: RebalanceCurveConfig
StablecoinFees -> hylo_idl :: exchange :: types :: StablecoinFees
YieldHarvestConfig -> hylo_idl :: exchange :: types :: YieldHarvestConfig
hylo_idl :: exchange :: types :: FeePair -> FeePair
hylo_idl :: exchange :: types :: FundingRateConfig -> FundingRateConfig
hylo_idl :: exchange :: types :: LevercoinFees -> LevercoinFees
hylo_idl :: exchange :: types :: RebalanceCurveConfig -> RebalanceCurveConfig
hylo_idl :: exchange :: types :: StablecoinFees -> StablecoinFees
hylo_idl :: exchange :: types :: YieldHarvestConfig -> YieldHarvestConfig

## Module Dependencies

account_builders                     -> (no internal deps)
account_builders::exchange           -> exchange, exchange::client::accounts, pda, stability_pool, tokens
account_builders::stability_pool     -> pda, stability_pool, stability_pool::client::accounts, tokens
account_metas                        -> (no internal deps)
asset_swap_config                    -> error::CoreError, fee_controller
codegen                              -> (no internal deps)
conversion                           -> error::CoreError, pyth
crate                                -> super::account_builders::exchange, super::account_builders::stability_pool, super::codegen::hylo_exchange, super::codegen::hylo_router, super::codegen::hylo_stability_pool, super::instruction_builders::exchange, super::instruction_builders::router, super::instruction_builders::stability_pool
error                                -> (no internal deps)
exchange                             -> (no internal deps)
exchange_client                      -> program_client, util
exchange_context                     -> conversion, error::CoreError, exchange_math, fee_controller, pyth, rebalance_math, rebalance_pricing, stability_mode
exchange_context::exo                -> conversion, error::CoreError, exchange_math, fee_controller, fee_curves, interpolated_fees, pyth, rebalance_pricing, solana_clock, stability_mode, super, super::validate_stability_thresholds, virtual_stablecoin
exchange_context::lst                -> conversion, error::CoreError, exchange_math, fee_controller, fee_curves, interpolated_fees, lst_sol_price, pyth, rebalance_pricing, solana_clock, stability_mode, super, super::validate_stability_thresholds, total_sol_cache, virtual_stablecoin
exchange_math                        -> error::CoreError, pyth
fee_controller                       -> error::CoreError, stability_mode::StabilityMode
fee_curves                           -> interp
funding_rate                         -> error::CoreError, fee_controller
idl_type_bridge                      -> fee_controller, funding_rate, lst_sol_price, rebalance_pricing, slippage_config, total_sol_cache, virtual_stablecoin, yields
instruction_builders                 -> (no internal deps)
instruction_builders::exchange       -> exchange, exchange::client, exchange::types, pda, stability_pool, tokens
instruction_builders::router         -> router, router::client
instruction_builders::stability_pool -> exchange, pda, stability_pool, stability_pool::client, stability_pool::types, tokens
interp                               -> error
interpolated_fees                    -> error, fee_controller, interp
jupiter                              -> account_metas, util
lst_sol_price                        -> error::CoreError, fee_controller
pda                                  -> exchange, stability_pool, tokens
prelude                              -> ComputeUnitInfo, ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER, ExecutableQuote, ExecutableQuoteValue, LST, Operation, ProtocolStateStrategy, QuoteMetadata, QuoteStrategy, RuntimeQuoteStrategy, SimulationStrategy, exchange_client, program_client, protocol_state, router_client, simulated_operation, stability_pool_client, token_operation, transaction
program_client                       -> util
protocol_state                       -> (no internal deps)
protocol_state::accounts             -> (no internal deps)
protocol_state::provider             -> protocol_state
protocol_state::state                -> LST, protocol_state
protocol_state_strategy              -> protocol_state, runtime_quote_strategy
protocol_state_strategy::router      -> ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER, ExecutableQuote, QuoteStrategy, protocol_state, protocol_state_strategy, token_operation
pyth                                 -> error::CoreError, solana_clock
quote_metadata                       -> (no internal deps)
quote_strategy                       -> ExecutableQuote
rebalance_math                       -> (no internal deps)
rebalance_pricing                    -> error, interp, pyth
router                               -> (no internal deps)
router_client                        -> program_client, transaction
router_client::instructions          -> super, util
router_client::transaction_data      -> program_client, super, transaction
runtime_quote_strategy               -> ExecutableQuoteValue, quote_metadata, quote_strategy
simulated_operation                  -> ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER, token_operation
simulated_operation::exchange        -> LST, Local, simulated_operation, token_operation
simulated_operation::stability_pool  -> simulated_operation, token_operation
simulation_strategy                  -> runtime_quote_strategy
simulation_strategy::router          -> ExecutableQuote, QuoteStrategy, simulated_operation, simulation_strategy
slippage_config                      -> error::CoreError, util
solana_clock                         -> (no internal deps)
spl_stake_pool                       -> error, lst_sol_price
stability_mode                       -> error::CoreError, stability_mode::StabilityMode
stability_pool                       -> (no internal deps)
stability_pool_client                -> program_client, util
stability_pool_math                  -> conversion, error::CoreError, fee_controller, pyth
token_operation                      -> (no internal deps)
token_operation::exchange            -> LST, Local, protocol_state, token_operation
token_operation::stability_pool      -> protocol_state, token_operation
tokens                               -> exchange, pda, stability_pool
total_sol_cache                      -> error::CoreError
transaction                          -> program_client
type_bridge                          -> (no internal deps)
util                                 -> error::CoreError, exchange_client, prelude, program_client, router_client, stability_pool_client
virtual_stablecoin                   -> error::CoreError
yields                               -> error::CoreError, fee_controller

## Key Types (referenced from 3+ modules)

UFix64                   — used in 28 modules
Pubkey                   — used in 26 modules
N9                       — used in 22 modules
N6                       — used in 16 modules
Exp                      — used in 13 modules
UFixValue64              — used in 13 modules
Integer                  — used in 11 modules
FeeExtract               — used in 10 modules
SolanaClock              — used in 10 modules
FeeExp                   — used in 9 modules
TokenMint                — used in 9 modules
Instruction              — used in 8 modules
PriceRange               — used in 8 modules
ProtocolState            — used in 8 modules
HYUSD                    — used in 7 modules
N2                       — used in 7 modules
HYLOSOL                  — used in 6 modules
IN                       — used in 6 modules
JITOSOL                  — used in 6 modules
LST                      — used in 6 modules
N4                       — used in 6 modules
OUT                      — used in 6 modules
OperationOutput          — used in 6 modules
RouterClient             — used in 6 modules
SwapOperationOutput      — used in 6 modules
VersionedTransactionData — used in 6 modules
Keypair                  — used in 5 modules
LevercoinFees            — used in 5 modules
Mint                     — used in 5 modules
OraclePrice              — used in 5 modules
ProgramClient            — used in 5 modules
RebalanceCurveConfig     — used in 5 modules
SHYUSD                   — used in 5 modules
SlippageConfig           — used in 5 modules
StabilityMode            — used in 5 modules
TokenOperation           — used in 5 modules
XSOL                     — used in 5 modules
CBBTC                    — used in 4 modules
Clock                    — used in 4 modules
FixInterp                — used in 4 modules
LstSolPrice              — used in 4 modules
LstSwapOperationOutput   — used in 4 modules
MAX_FEE                  — used in 4 modules
MintOperationOutput      — used in 4 modules
PROGRAM_ID               — used in 4 modules
PriceUpdateV2            — used in 4 modules
Program                  — used in 4 modules
ProtocolAccounts         — used in 4 modules
RedeemOperationOutput    — used in 4 modules
RuntimeQuoteStrategy     — used in 4 modules
Signature                — used in 4 modules
SimulatedOperation       — used in 4 modules
StabilityController      — used in 4 modules
StateProvider            — used in 4 modules
Sync                     — used in 4 modules
TokenMetadata            — used in 4 modules
TransactionSyntax        — used in 4 modules
USDC                     — used in 4 modules
UpdateAdmin              — used in 4 modules
VirtualStablecoin        — used in 4 modules
XBTC                     — used in 4 modules
AccountMeta              — used in 3 modules
AnchorDeserialize        — used in 3 modules
BuildTransactionData     — used in 3 modules
ClockRef                 — used in 3 modules
Cluster                  — used in 3 modules
ComputeUnitInfo          — used in 3 modules
ComputeUnitStrategy      — used in 3 modules
ConvertLeverToStableExo  — used in 3 modules
ConvertLeverToStableLst  — used in 3 modules
ConvertStableToLeverExo  — used in 3 modules
ConvertStableToLeverLst  — used in 3 modules
Error                    — used in 3 modules
Event                    — used in 3 modules
ExchangeClient           — used in 3 modules
ExchangeContext          — used in 3 modules
ExecutableQuote          — used in 3 modules
ExoExchangeContext       — used in 3 modules
From                     — used in 3 modules
ID                       — used in 3 modules
IFix64                   — used in 3 modules
InExp                    — used in 3 modules
InitializeUsdc           — used in 3 modules
Inputs                   — used in 3 modules
InterpolatedMintFees     — used in 3 modules
InterpolatedRedeemFees   — used in 3 modules
Local                    — used in 3 modules
LstExchangeContext       — used in 3 modules
MintLevercoinExo         — used in 3 modules
MintLevercoinLst         — used in 3 modules
MintStablecoinExo        — used in 3 modules
MintStablecoinLst        — used in 3 modules
MintStablecoinUsdc       — used in 3 modules
N5                       — used in 3 modules
N8                       — used in 3 modules
Operation                — used in 3 modules
OracleConfig             — used in 3 modules
ProtocolStateStrategy    — used in 3 modules
QuoteMetadata            — used in 3 modules
QuoteStrategy            — used in 3 modules
RedeemLevercoinExo       — used in 3 modules
RedeemLevercoinLst       — used in 3 modules
RedeemStablecoinExo      — used in 3 modules
RedeemStablecoinLst      — used in 3 modules
RedeemStablecoinUsdc     — used in 3 modules
RegisterExo              — used in 3 modules
RouterArgs               — used in 3 modules
RpcStateProvider         — used in 3 modules
Send                     — used in 3 modules
SimulationStrategy       — used in 3 modules
StabilityPoolClient      — used in 3 modules
SwapExoToUsdc            — used in 3 modules
SwapLstToLst             — used in 3 modules
SwapLstToUsdc            — used in 3 modules
SwapUsdcToExo            — used in 3 modules
SwapUsdcToLst            — used in 3 modules
ToAccountMetas           — used in 3 modules
TotalSolCache            — used in 3 modules
TryFrom                  — used in 3 modules
UpdateLstRebalanceFee    — used in 3 modules

