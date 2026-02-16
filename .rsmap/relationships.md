## Trait Implementations

Amm                                                          <- HyloJupiterPair < IN , OUT >
AsRef < str >                                                <- Operation
BuildTransactionData < HYUSD , OUT >                         <- ExchangeClient
BuildTransactionData < HYUSD , SHYUSD >                      <- StabilityPoolClient
BuildTransactionData < HYUSD , XSOL >                        <- ExchangeClient
BuildTransactionData < IN , HYUSD >                          <- ExchangeClient
BuildTransactionData < IN , XSOL >                           <- ExchangeClient
BuildTransactionData < L1 , L2 >                             <- ExchangeClient
BuildTransactionData < SHYUSD , HYUSD >                      <- StabilityPoolClient
BuildTransactionData < SHYUSD , L >                          <- SimulationStrategy
BuildTransactionData < SHYUSD , OUT >                        <- StabilityPoolClient
BuildTransactionData < XSOL , HYUSD >                        <- ExchangeClient
BuildTransactionData < XSOL , OUT >                          <- ExchangeClient
Clone                                                        <- HyloJupiterPair < IN , OUT >
Default                                                      <- ComputeUnitInfo, VirtualStablecoin
Display                                                      <- StabilityMode
ExchangeContext                                              <- ExoExchangeContext < C >, LstExchangeContext < C >
FeeController                                                <- LevercoinFees, StablecoinFees
From < ExecutableQuote < InExp , OutExp , FeeExp > >         <- ExecutableQuoteValue
From < SlippageConfig >                                      <- hylo_idl :: exchange :: types :: SlippageConfig
From < UFixValue64 >                                         <- crate :: exchange :: types :: UFixValue64
From < crate :: exchange :: types :: UFixValue64 >           <- UFixValue64
From < crate :: stability_pool :: types :: UFixValue64 >     <- UFixValue64
From < hylo_idl :: exchange :: types :: FeePair >            <- FeePair
From < hylo_idl :: exchange :: types :: HarvestCache >       <- HarvestCache
From < hylo_idl :: exchange :: types :: LevercoinFees >      <- LevercoinFees
From < hylo_idl :: exchange :: types :: LstSolPrice >        <- LstSolPrice
From < hylo_idl :: exchange :: types :: StablecoinFees >     <- StablecoinFees
From < hylo_idl :: exchange :: types :: TotalSolCache >      <- TotalSolCache
From < hylo_idl :: exchange :: types :: VirtualStablecoin >  <- VirtualStablecoin
From < hylo_idl :: exchange :: types :: YieldHarvestConfig > <- YieldHarvestConfig
InstructionBuilder < HYUSD , L >                             <- ExchangeInstructionBuilder
InstructionBuilder < HYUSD , SHYUSD >                        <- StabilityPoolInstructionBuilder
InstructionBuilder < HYUSD , XSOL >                          <- ExchangeInstructionBuilder
InstructionBuilder < L , HYUSD >                             <- ExchangeInstructionBuilder
InstructionBuilder < L , XSOL >                              <- ExchangeInstructionBuilder
InstructionBuilder < L1 , L2 >                               <- ExchangeInstructionBuilder
InstructionBuilder < SHYUSD , HYUSD >                        <- StabilityPoolInstructionBuilder
InstructionBuilder < XSOL , HYUSD >                          <- ExchangeInstructionBuilder
InstructionBuilder < XSOL , L >                              <- ExchangeInstructionBuilder
InstructionBuilderExt                                        <- X
InterpolatedFeeController                                    <- InterpolatedMintFees, InterpolatedRedeemFees
LST                                                          <- HYLOSOL, JITOSOL
Local                                                        <- HYLOSOL, JITOSOL
PairConfig < HYLOSOL , HYUSD >                               <- HyloJupiterPair < HYLOSOL , HYUSD >
PairConfig < HYLOSOL , XSOL >                                <- HyloJupiterPair < HYLOSOL , XSOL >
PairConfig < HYUSD , SHYUSD >                                <- HyloJupiterPair < HYUSD , SHYUSD >
PairConfig < HYUSD , XSOL >                                  <- HyloJupiterPair < HYUSD , XSOL >
PairConfig < JITOSOL , HYUSD >                               <- HyloJupiterPair < JITOSOL , HYUSD >
PairConfig < JITOSOL , XSOL >                                <- HyloJupiterPair < JITOSOL , XSOL >
ProgramClient                                                <- ExchangeClient, StabilityPoolClient
QuoteStrategy < HYUSD , L , C >                              <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < HYUSD , SHYUSD , C >                         <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < HYUSD , XSOL , C >                           <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < L , HYUSD , C >                              <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < L , XSOL , C >                               <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < L1 , L2 , C >                                <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < SHYUSD , HYUSD , C >                         <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < SHYUSD , L , C >                             <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < XSOL , HYUSD , C >                           <- ProtocolStateStrategy < S >, SimulationStrategy
QuoteStrategy < XSOL , L , C >                               <- ProtocolStateStrategy < S >, SimulationStrategy
RuntimeQuoteStrategy < C >                                   <- ProtocolStateStrategy < S >
RuntimeQuoteStrategy < Clock >                               <- SimulationStrategy
SimulatedOperation < HYUSD , L >                             <- ExchangeClient
SimulatedOperation < HYUSD , SHYUSD >                        <- StabilityPoolClient
SimulatedOperation < HYUSD , XSOL >                          <- ExchangeClient
SimulatedOperation < L , HYUSD >                             <- ExchangeClient
SimulatedOperation < L , XSOL >                              <- ExchangeClient
SimulatedOperation < L1 , L2 >                               <- ExchangeClient
SimulatedOperation < SHYUSD , HYUSD >                        <- StabilityPoolClient
SimulatedOperation < XSOL , HYUSD >                          <- ExchangeClient
SimulatedOperation < XSOL , L >                              <- ExchangeClient
SimulatedOperationExt                                        <- X
SolanaClock                                                  <- Clock, ClockRef
StateProvider < C >                                          <- std :: sync :: Arc < T >
StateProvider < Clock >                                      <- RpcStateProvider
TokenMint                                                    <- HYLOSOL, HYUSD, JITOSOL, SHYUSD, XSOL
TokenOperation < HYUSD , L >                                 <- ProtocolState < C >
TokenOperation < HYUSD , SHYUSD >                            <- ProtocolState < C >
TokenOperation < HYUSD , XSOL >                              <- ProtocolState < C >
TokenOperation < L , HYUSD >                                 <- ProtocolState < C >
TokenOperation < L , XSOL >                                  <- ProtocolState < C >
TokenOperation < L1 , L2 >                                   <- ProtocolState < C >
TokenOperation < SHYUSD , HYUSD >                            <- ProtocolState < C >
TokenOperation < SHYUSD , L >                                <- ProtocolState < C >
TokenOperation < XSOL , HYUSD >                              <- ProtocolState < C >
TokenOperation < XSOL , L >                                  <- ProtocolState < C >
TokenOperationExt                                            <- X
TransactionSyntax                                            <- ExchangeClient, SimulationStrategy, StabilityPoolClient
TryFrom < & ProtocolAccounts >                               <- ProtocolState < Clock >
TryFrom < (& [Pubkey] , & [Option < Account >]) >            <- ProtocolAccounts
TryFrom < crate :: exchange :: types :: UFixValue64 >        <- UFix64 < Exp >
TryFrom < crate :: stability_pool :: types :: UFixValue64 >  <- UFix64 < Exp >
std :: fmt :: Display                                        <- Operation

## Error Chains

ExecutableQuote < InExp , OutExp , FeeExp > -> ExecutableQuoteValue
SlippageConfig -> hylo_idl :: exchange :: types :: SlippageConfig
crate :: stability_pool :: types :: UFixValue64 -> UFixValue64 -> crate :: exchange :: types :: UFixValue64
hylo_idl :: exchange :: types :: FeePair -> FeePair
hylo_idl :: exchange :: types :: HarvestCache -> HarvestCache
hylo_idl :: exchange :: types :: LevercoinFees -> LevercoinFees
hylo_idl :: exchange :: types :: LstSolPrice -> LstSolPrice
hylo_idl :: exchange :: types :: StablecoinFees -> StablecoinFees
hylo_idl :: exchange :: types :: TotalSolCache -> TotalSolCache
hylo_idl :: exchange :: types :: VirtualStablecoin -> VirtualStablecoin
hylo_idl :: exchange :: types :: YieldHarvestConfig -> YieldHarvestConfig

## Module Dependencies

account_builders                        -> (no internal deps)
account_builders::exchange              -> ata, exchange::client::accounts, pda, stability_pool, tokens
account_builders::stability_pool        -> pda, stability_pool::client::accounts, tokens
account_metas                           -> (no internal deps)
codegen                                 -> (no internal deps)
conversion                              -> error::CoreError, pyth
crate                                   -> super::account_builders::exchange, super::account_builders::stability_pool, super::codegen::hylo_exchange, super::codegen::hylo_stability_pool, super::instruction_builders::exchange, super::instruction_builders::stability_pool
error                                   -> (no internal deps)
exchange                                -> (no internal deps)
exchange_client                         -> instructions, program_client, syntax_helpers, transaction, util
exchange_context                        -> conversion, error::CoreError, exchange_math, fee_controller, pyth, stability_mode, stability_pool_math
exchange_context::exo                   -> conversion, error::CoreError, exchange_math, fee_controller, fee_curves, interpolated_fees, pyth, solana_clock, stability_mode, super, virtual_stablecoin
exchange_context::lst                   -> conversion, error::CoreError, exchange_math, fee_controller, lst_sol_price, pyth, solana_clock, stability_mode, super, total_sol_cache, virtual_stablecoin
exchange_math                           -> error::CoreError, pyth
fee_controller                          -> error::CoreError, stability_mode::StabilityMode
fee_curves                              -> interp
funding_rate                            -> error::CoreError, fee_controller
idl_type_bridge                         -> fee_controller, lst_sol_price, slippage_config, total_sol_cache, virtual_stablecoin, yields
instruction_builders                    -> (no internal deps)
instruction_builders::exchange          -> exchange, exchange::client, pda, stability_pool, tokens
instruction_builders::stability_pool    -> exchange, pda, stability_pool, stability_pool::client, tokens
instructions                            -> transaction, util
interp                                  -> error
interpolated_fees                       -> error, fee_controller, interp
jupiter                                 -> account_metas, util
lst_sol_price                           -> error::CoreError
lst_swap_config                         -> error::CoreError, fee_controller
pda                                     -> exchange, stability_pool, tokens
prelude                                 -> ComputeUnitInfo, ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER, ExecutableQuote, ExecutableQuoteValue, LST, Operation, ProtocolStateStrategy, QuoteMetadata, QuoteStrategy, RuntimeQuoteStrategy, SimulationStrategy, exchange_client, instructions, program_client, protocol_state, simulated_operation, stability_pool_client, syntax_helpers, token_operation, transaction
program_client                          -> util
protocol_state                          -> (no internal deps)
protocol_state::accounts                -> (no internal deps)
protocol_state::provider                -> protocol_state
protocol_state::state                   -> LST, protocol_state
protocol_state_strategy                 -> protocol_state, runtime_quote_strategy
protocol_state_strategy::exchange       -> ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER, ExecutableQuote, LST, Local, QuoteStrategy, protocol_state, protocol_state_strategy, token_operation
protocol_state_strategy::stability_pool -> ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER, DEFAULT_CUS_WITH_BUFFER_X3, ExecutableQuote, LST, Local, QuoteStrategy, protocol_state, protocol_state_strategy, token_operation
pyth                                    -> error::CoreError, solana_clock
quote_metadata                          -> (no internal deps)
quote_strategy                          -> ExecutableQuote
runtime_quote_strategy                  -> ExecutableQuoteValue, quote_metadata, quote_strategy
simulated_operation                     -> ComputeUnitStrategy, DEFAULT_CUS_WITH_BUFFER, token_operation
simulated_operation::exchange           -> LST, Local, simulated_operation, token_operation
simulated_operation::stability_pool     -> simulated_operation, token_operation
simulation_strategy                     -> runtime_quote_strategy
simulation_strategy::exchange           -> ExecutableQuote, Local, QuoteStrategy, simulated_operation, simulation_strategy
simulation_strategy::stability_pool     -> ExecutableQuote, Local, QuoteStrategy, simulated_operation, simulation_strategy
slippage_config                         -> error::CoreError
solana_clock                            -> (no internal deps)
stability_mode                          -> error::CoreError, stability_mode::StabilityMode
stability_pool                          -> (no internal deps)
stability_pool_client                   -> exchange_client, instructions, program_client, syntax_helpers, transaction, util
stability_pool_math                     -> conversion, error::CoreError, fee_controller, pyth
syntax_helpers                          -> instructions
token_operation                         -> (no internal deps)
token_operation::exchange               -> LST, Local, protocol_state, token_operation
token_operation::stability_pool         -> LST, Local, protocol_state, token_operation
tokens                                  -> (no internal deps)
total_sol_cache                         -> error::CoreError
transaction                             -> program_client
type_bridge                             -> (no internal deps)
util                                    -> exchange_client, prelude, program_client, stability_pool_client
virtual_stablecoin                      -> error::CoreError
yields                                  -> error::CoreError, fee_controller

## Key Types (referenced from 3+ modules)

Pubkey                   — used in 26 modules
UFix64                   — used in 24 modules
N9                       — used in 21 modules
N6                       — used in 18 modules
Exp                      — used in 14 modules
FeeExp                   — used in 14 modules
HYUSD                    — used in 14 modules
LST                      — used in 14 modules
SolanaClock              — used in 14 modules
Integer                  — used in 10 modules
TokenMint                — used in 10 modules
UFixValue64              — used in 10 modules
FeeExtract               — used in 9 modules
OUT                      — used in 9 modules
ProtocolState            — used in 9 modules
SHYUSD                   — used in 9 modules
XSOL                     — used in 9 modules
IN                       — used in 8 modules
Local                    — used in 8 modules
ExecutableQuote          — used in 7 modules
Inputs                   — used in 7 modules
Instruction              — used in 7 modules
PriceRange               — used in 7 modules
QuoteStrategy            — used in 7 modules
VersionedTransactionData — used in 7 modules
BuildTransactionData     — used in 6 modules
ExchangeClient           — used in 6 modules
L1                       — used in 6 modules
L2                       — used in 6 modules
N4                       — used in 6 modules
N8                       — used in 6 modules
StateProvider            — used in 6 modules
SwapOperationOutput      — used in 6 modules
TokenOperation           — used in 6 modules
HYLOSOL                  — used in 5 modules
JITOSOL                  — used in 5 modules
LevercoinFees            — used in 5 modules
N2                       — used in 5 modules
ProtocolStateStrategy    — used in 5 modules
RedeemOperationOutput    — used in 5 modules
SimulationStrategy       — used in 5 modules
StabilityMode            — used in 5 modules
StabilityPoolArgs        — used in 5 modules
StabilityPoolClient      — used in 5 modules
TransactionSyntax        — used in 5 modules
Clock                    — used in 4 modules
InExp                    — used in 4 modules
Keypair                  — used in 4 modules
LstSwapOperationOutput   — used in 4 modules
MintArgs                 — used in 4 modules
MintOperationOutput      — used in 4 modules
OperationOutput          — used in 4 modules
PriceUpdateV2            — used in 4 modules
ProgramClient            — used in 4 modules
ProtocolAccounts         — used in 4 modules
RedeemArgs               — used in 4 modules
RuntimeQuoteStrategy     — used in 4 modules
Signature                — used in 4 modules
SimulatedOperation       — used in 4 modules
StabilityController      — used in 4 modules
SwapArgs                 — used in 4 modules
Sync                     — used in 4 modules
VirtualStablecoin        — used in 4 modules
AccountMeta              — used in 3 modules
ClockRef                 — used in 3 modules
Cluster                  — used in 3 modules
ComputeUnitInfo          — used in 3 modules
ComputeUnitStrategy      — used in 3 modules
Error                    — used in 3 modules
Event                    — used in 3 modules
ExchangeContext          — used in 3 modules
FixInterp                — used in 3 modules
From                     — used in 3 modules
InstructionBuilder       — used in 3 modules
LstExchangeContext       — used in 3 modules
LstSolPrice              — used in 3 modules
LstSwapArgs              — used in 3 modules
Mint                     — used in 3 modules
MintLevercoin            — used in 3 modules
MintStablecoin           — used in 3 modules
Operation                — used in 3 modules
OracleConfig             — used in 3 modules
OutExp                   — used in 3 modules
PROGRAM_ID               — used in 3 modules
Program                  — used in 3 modules
QuoteMetadata            — used in 3 modules
RedeemLevercoin          — used in 3 modules
RedeemStablecoin         — used in 3 modules
RpcStateProvider         — used in 3 modules
Send                     — used in 3 modules
SlippageConfig           — used in 3 modules
StablecoinFees           — used in 3 modules
SwapLeverToStable        — used in 3 modules
SwapStableToLever        — used in 3 modules
TotalSolCache            — used in 3 modules
TryFrom                  — used in 3 modules

