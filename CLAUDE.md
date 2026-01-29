# Hylo SDK

Rust SDK for the Hylo Protocol - a collateralized stablecoin and leverage token DEX on Solana.

## Development Environment

This project uses Nix for reproducible development environments.

```bash
# Enter dev shell (stable Rust 1.88.0)
nix develop

# Enter nightly shell (for polish/lint scripts)
nix develop .#nightly
```

### Commands

After completing ANY coding task, ALWAYS run:
```bash
nix develop .#nightly --command ./bin/polish.sh  # Format + auto-fix clippy
nix develop .#nightly --command ./bin/lint.sh    # Verify (CI check mode)
```

Other commands:
```bash
cargo test                    # Run all tests
cargo test -p hylo-quotes     # Run tests for specific crate
cargo +nightly udeps          # Check unused dependencies (nightly shell)
```

Integration tests require both env vars (`RPC_WS_URL` can be fake):
```bash
RPC_URL=https://mainnet.helius-rpc.com/?api-key=<key>
RPC_WS_URL=wss://mainnet.helius-rpc.com/?api-key=<key>
```

## Crates

| Crate | Purpose |
|-------|---------|
| `hylo-core` | Pure protocol math and types (no RPC) |
| `hylo-idl` | Anchor IDL and type-safe token definitions |
| `hylo-clients` | Transaction builders and execution clients |
| `hylo-quotes` | High-level quoting strategies |
| `hylo-jupiter` | Jupiter AMM integration |

## Code Style

From `rustfmt.toml`:
- 2-space indentation
- 80 character line width
- Import grouping: std → external → internal

Naming:
- Modules/functions: `snake_case`
- Structs/Traits: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Generic type params: Single letters (`IN`, `OUT`, `C`, `L`)

### Style Rules (IMPORTANT)

- NEVER use explicit `return` statements - use Rust's expression-oriented style
- NEVER use `for`/`while` loops - always prefer iterator API (`.map()`, `.filter()`, `.fold()`, etc.)
- NEVER use `.unwrap()` or `.expect()` - always use `?` error propagation
- Favor functional style over imperative control flow

### Documentation

- Module docs: `//!` with sections
- Function docs: `///` with `# Errors` section

### Error Handling

- Use `anyhow::Result<T>` for all fallible functions
- Custom errors in `hylo-core/src/error.rs`

## Architecture

### Type-Safe Token System

The `TokenMint` trait defines tokens with an associated `Exp` type for decimal precision:
- Tokens: `HYUSD`, `XSOL`, `SHYUSD`, `JITOSOL`, `HYLOSOL`
- Operations are generic over token pairs: `<IN: TokenMint, OUT: TokenMint>`

### Core Traits

| Trait | Purpose |
|-------|---------|
| `TokenOperation<IN, OUT>` | Pure math for computing quotes |
| `InstructionBuilder<IN, OUT>` | Build instructions for token pairs |
| `BuildTransactionData<IN, OUT>` | Full transaction construction |
| `SimulatePrice<IN, OUT>` | Quote via transaction simulation |
| `ProgramClient` | Base trait for exchange/stability pool clients |
| `RuntimeQuoteStrategy` | Unified quoting interface |

### Quoting Strategies

- `ProtocolStateStrategy` - Fast quotes from cached state (no RPC validation)
- `SimulationStrategy` - Slower but validates via RPC simulation

### Fixed-Point Math

Uses `hylo-fix` crate with `UFix64<Exp>` type:
- Compile-time decimal precision via type parameter (`N6`, `N9`)
- Import via `fix::prelude::*`

## Project Structure

```
sdk/
├── hylo-core/           # Pure protocol math (no RPC)
│   └── src/
│       ├── exchange_math.rs
│       ├── exchange_context.rs
│       ├── fee_controller.rs
│       └── pyth.rs
├── hylo-idl/            # Anchor IDL + type-safe tokens
│   └── src/
│       ├── tokens.rs    # TokenMint trait + HYUSD, XSOL, etc.
│       └── pda.rs       # Program-derived addresses
├── hylo-clients/        # Transaction builders
│   └── src/
│       ├── exchange_client.rs
│       ├── instructions.rs
│       ├── syntax_helpers.rs
│       └── util.rs
├── hylo-quotes/         # High-level quoting
│   └── src/
│       ├── protocol_state/
│       ├── protocol_state_strategy/
│       ├── simulated_operation/
│       ├── simulation_strategy/
│       └── token_operation/
└── hylo-jupiter/        # Jupiter AMM integration
```

## Common Patterns

Building instructions (type-safe, no RPC):
```rust
use hylo_clients::instructions::ExchangeInstructionBuilder as ExchangeIB;

let instructions = ExchangeIB::build_instructions::<JITOSOL, HYUSD>(args)?;
let luts = ExchangeIB::lookup_tables::<JITOSOL, HYUSD>();
```

Computing quotes from protocol state:
```rust
let output = state.compute_quote::<JITOSOL, HYUSD>(amount_in)?;
```

Using the prelude:
```rust
use hylo_clients::prelude::*;  // Common imports
use hylo_core::prelude::*;     // Math types
```

## Testing

- Integration tests in `tests/integration_tests.rs`
- Test context pattern: `TestContext::new().await?`
- Do NOT use `flaky_test` macro (deprecated)

## Notes

- All token operations are compile-time type-checked
- `UFix64` values use `.bits` for raw u64 access
- Slippage is in basis points (50 = 0.5%)
- Default compute units: 100k with buffer
- Jupiter integration uses `HyloJupiterPair<IN, OUT>` generic
