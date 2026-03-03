# Crate: hylo_clients (lib)
Edition: 2021
Version: 0.5.0
External deps: anchor-client, anchor-lang, anchor-spl, anyhow, async-trait, base64, bincode, futures, hylo-core, hylo-fix, hylo-idl, itertools, mpl-token-metadata, pyth-solana-receiver-sdk, serde_json, solana-address-lookup-table-interface, solana-rpc-client-api, tokio

## Module Tree
- crate — # Hylo Clients
  - exchange_client — RPC client for executing exchange program transactions including mint, redeem, swap, and admin operations.
  - instructions — Statically type-safe instruction building without requiring client
  - prelude — Convenience re-exports of commonly used types, traits, and token definitions.
  - program_client — Base trait for Anchor program clients with transaction building, simulation, and RPC helpers.
  - stability_pool_client — RPC client for executing stability pool transactions including deposit, withdraw, and rebalance.
  - syntax_helpers — Extension traits for cleaner static dispatch syntax.
  - transaction — Transaction argument types and traits for building and executing versioned transactions.
  - util — Shared utilities: lookup table helpers, test client builders, LST marker trait, and assertion macros.

# Crate: hylo_core (lib)
Edition: 2021
Version: 0.5.0
External deps: anchor-lang, anchor-spl, hylo-fix, hylo-idl, hylo-jupiter-amm-interface, itertools, pyth-solana-receiver-sdk, serde

## Module Tree
- crate — Root module for each crate in the Hylo Protocol SDK workspace.
  - conversion — Price conversion utilities between LST collateral, exogenous collateral, and protocol tokens (hyUSD, xSOL).
  - error — Custom error types for hylo-core protocol math and validation failures.
  - exchange_context — Exchange context trait and implementations.
    - exo — Exchange context implementation for exogenous (non-SOL) collateral pairs with interpolated fee curves.
    - lst — Exchange context implementation for SOL/LST collateral pairs using the TotalSolCache and Pyth oracle.
  - exchange_math — Pure math functions for collateral ratio, TVL, stablecoin/levercoin NAV, and mint/swap capacity.
  - fee_controller — Fee configuration types and trait that select mint/redeem fees based on the current stability mode.
  - fee_curves — Defines piecewise-linear fee curves for collateral-ratio-dependent mint and redeem fees.
  - funding_rate — Per-epoch funding rate applied to exogenous collateral without native yield to cover protocol costs.
  - idl_type_bridge — From impls converting Anchor IDL-generated types into hylo-core domain types.
  - interp — Piecewise linear interpolation for fee curves.
  - interpolated_fees — Fee controllers that use interpolated curves to compute collateral-ratio-dependent fees for exo pairs.
  - lst_sol_price — Tracks LST-to-SOL exchange rate per epoch for accurate collateral valuation.
  - lst_swap_config — Configuration and fee logic for direct LST-to-LST swaps within the exchange.
  - pyth — Pyth oracle integration with price validation, confidence checks, and staleness guards.
  - rebalance_pricing — Oracle-derived collateral rebalancing price curves.
  - slippage_config — Client-specified slippage tolerance paired with expected output amount for transaction validation.
  - solana_clock — Abstraction trait over Solana's on-chain Clock sysvar for testability.
  - stability_mode — Defines protocol stability modes (Normal, Mode1, Mode2, Depeg) based on collateral ratio thresholds.
  - stability_pool_math — Pure math for LP token NAV, pool capacity, withdrawal fees, and rebalancing targets.
  - total_sol_cache — Epoch-validated cache of total SOL collateral held by the exchange, updated on deposits and withdrawals.
  - util — Shared utilities: lookup table helpers, test client builders, LST marker trait, and assertion macros.
  - virtual_stablecoin — Counter tracking the supply of virtual stablecoin for exogenous collateral pairs without a real mint.
  - yields — Yield harvest configuration (allocation and fee percentages) and epoch-based harvest caching.

# Crate: hylo_idl (lib)
Edition: 2021
Version: 0.5.0
External deps: anchor-lang, anchor-spl, anyhow, hylo-fix, mpl-token-metadata, solana-address-lookup-table-interface, solana-loader-v3-interface

## Module Tree
- crate — Root module for each crate in the Hylo Protocol SDK workspace.
  - codegen — Auto-generated Anchor IDL code for the exchange and stability pool programs.
  - account_builders — Parent module for Anchor account context builders used by exchange and stability pool instructions.
    - exchange — Builds Anchor account contexts for exchange operations like minting, redeeming, and swapping tokens.
    - stability_pool — Builds Anchor account contexts for stability pool deposit and withdrawal operations.
  - instruction_builders — Parent module for low-level Solana instruction builders for exchange and stability pool programs.
    - exchange — Instruction builders for Hylo Exchange.
    - stability_pool — Instruction builders for Hylo Stability Pool.
  - exchange — Re-exports exchange program account builders, IDL codegen, and instruction builders.
  - stability_pool — Re-exports stability pool program account builders, IDL codegen, and instruction builders.
  - pda — Program Derived Address constants and derivation functions for all protocol accounts.
  - tokens — Type-safe token definitions (HYUSD, XSOL, SHYUSD, JITOSOL, HYLOSOL) with mint addresses and decimal precision.
  - type_bridge — Conversions between IDL-generated UFixValue64 and hylo-fix's UFix64 across both programs.

# Crate: hylo_jupiter (lib)
Edition: 2021
Version: 0.5.0
External deps: anchor-client, anchor-lang, anchor-spl, anyhow, bincode, hylo-clients, hylo-core, hylo-fix, hylo-idl, hylo-jupiter-amm-interface, hylo-quotes, pyth-solana-receiver-sdk, rust_decimal, solana-rpc-client, tokio

## Module Tree
- crate — Root module for each crate in the Hylo Protocol SDK workspace.
  - account_metas — Creates Jupiter-compatible SwapAndAccountMetas for each Hylo operation type.
  - jupiter — Jupiter AMM trait implementation allowing Hylo pairs to be routed through Jupiter aggregator.
  - util — Shared utilities: lookup table helpers, test client builders, LST marker trait, and assertion macros.

# Crate: hylo_quotes (lib)
Edition: 2021
Version: 0.5.0
External deps: anchor-client, anchor-lang, anchor-spl, anyhow, async-trait, bincode, hylo-clients, hylo-core, hylo-fix, hylo-idl, pyth-solana-receiver-sdk, serde, solana-rpc-client, tokio

## Module Tree
- crate — Type-safe quote computation and transaction building for the Hylo protocol.
  - prelude — Common imports for hylo-quotes.
  - protocol_state — Aggregates on-chain protocol state from multiple accounts into a single typed snapshot.
    - accounts — Type-safe collection of protocol state accounts
    - provider — State provider trait and implementations
    - state — Protocol state types and deserialization
  - protocol_state_strategy — Quote strategy using protocol state.
    - exchange — `QuoteStrategy` implementations for exchange pairs using `TokenOperation`.
    - stability_pool — `QuoteStrategy` implementations for stability pool pairs using
  - quote_metadata — Quote metadata types
  - quote_strategy — Core trait for computing executable quotes for any typed token pair operation.
  - runtime_quote_strategy — Macro-generated trait that dispatches runtime Pubkey pairs to compile-time typed QuoteStrategy impls.
  - simulated_operation — Extract quote data from simulation events.
    - exchange — `SimulatedOperation` implementations for exchange pairs.
    - stability_pool — `SimulatedOperation` implementations for stability pool pairs.
  - simulation_strategy — Quote strategy using transaction simulation.
    - exchange — Exchange pair QuoteStrategy impls for SimulationStrategy using on-chain simulation.
    - stability_pool — Stability pool QuoteStrategy impls for SimulationStrategy using on-chain simulation.
  - token_operation — Token operation trait for pure protocol math.
    - exchange — `TokenOperation` implementations for exchange pairs.
    - stability_pool — `TokenOperation` implementations for stability pool pairs.

