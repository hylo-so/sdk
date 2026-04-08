# Shadow Deployment Feature Flag Plan

Feature flag `shadow` that swaps program IDs, token mints, and lookup
tables at compile time. One set of IDL files, zero duplication.

## Values Needed

| # | Value | Destination |
|---|-------|-------------|
| 1 | Shadow exchange program ID | `hylo-idl/src/lib.rs` |
| 2 | Shadow stability pool program ID | `hylo-idl/src/lib.rs` |
| 3 | Shadow HYUSD mint | `hylo-idl/src/tokens.rs` |
| 4 | Shadow SHYUSD mint | `hylo-idl/src/tokens.rs` |
| 5 | Shadow XSOL mint | `hylo-idl/src/tokens.rs` |
| 6 | Shadow exchange lookup table | `hylo-clients/src/util.rs` |
| 7 | Shadow stability pool lookup table | `hylo-clients/src/util.rs` |
| 8 | Shadow reference wallet | `hylo-clients/src/util.rs` |

## Steps

### 1. Thread `shadow` feature through Cargo.toml files

```toml
# hylo-idl/Cargo.toml
[features]
shadow = []

# hylo-core/Cargo.toml
[features]
shadow = ["hylo-idl?/shadow"]

# hylo-clients/Cargo.toml
[features]
shadow = ["hylo-idl/shadow", "hylo-core/shadow"]

# hylo-quotes/Cargo.toml
[features]
shadow = ["hylo-idl/shadow", "hylo-core/shadow", "hylo-clients/shadow"]

# hylo-jupiter/Cargo.toml
[features]
shadow = ["hylo-idl/shadow", "hylo-core/shadow", "hylo-clients/shadow", "hylo-quotes/shadow"]
```

### 2. Override program IDs in `hylo-idl/src/lib.rs`

Rust rule: an explicit item in a module shadows a glob import. The
`declare_program!` macro brings in `ID` via glob, so we just define
a conditional `const ID` that wins when `shadow` is active.

```rust
pub mod exchange {
  pub use super::account_builders::exchange as account_builders;
  pub use super::codegen::hylo_exchange::*;
  pub use super::instruction_builders::exchange as instruction_builders;

  #[cfg(feature = "shadow")]
  pub const ID: Pubkey = pubkey!("<SHADOW_EXCHANGE_PROGRAM_ID>");
}

pub mod stability_pool {
  pub use super::account_builders::stability_pool as account_builders;
  pub use super::codegen::hylo_stability_pool::*;
  pub use super::instruction_builders::stability_pool as instruction_builders;

  #[cfg(feature = "shadow")]
  pub const ID: Pubkey = pubkey!("<SHADOW_STABILITY_POOL_PROGRAM_ID>");
}
```

Requires adding to the top of `lib.rs`:

```rust
#[cfg(feature = "shadow")]
use anchor_lang::solana_program::pubkey;
#[cfg(feature = "shadow")]
use anchor_lang::prelude::Pubkey;
```

### 3. Feature-gate token mints in `hylo-idl/src/tokens.rs`

Only protocol-minted tokens change. JITOSOL and HYLOSOL are external.

```rust
impl TokenMint for HYUSD {
  type Exp = N6;
  #[cfg(not(feature = "shadow"))]
  const MINT: Pubkey = pubkey!("6hxiteDeaUt1mVHapfZCnLcZ6wTogQWLE3zNcuTBaZNL");
  #[cfg(feature = "shadow")]
  const MINT: Pubkey = pubkey!("<SHADOW_HYUSD_MINT>");
}

impl TokenMint for SHYUSD {
  type Exp = N6;
  #[cfg(not(feature = "shadow"))]
  const MINT: Pubkey = pubkey!("AayEwKC9oN2vAX7sTQNPLMGnebdEzizQHhdL7ebvhw4i");
  #[cfg(feature = "shadow")]
  const MINT: Pubkey = pubkey!("<SHADOW_SHYUSD_MINT>");
}

impl TokenMint for XSOL {
  type Exp = N6;
  #[cfg(not(feature = "shadow"))]
  const MINT: Pubkey = pubkey!("6LBhoz13JunysSHJZVmCiMYrVhfR4r25q28ft2oHNGcH");
  #[cfg(feature = "shadow")]
  const MINT: Pubkey = pubkey!("<SHADOW_XSOL_MINT>");
}
```

### 4. Feature-gate lookup tables in `hylo-clients/src/util.rs`

Replace the three bare constants with a feature-gated inner module:

```rust
#[cfg(not(feature = "shadow"))]
mod deployment {
  use anchor_client::solana_sdk::{pubkey, pubkey::Pubkey};

  pub const EXCHANGE_LOOKUP_TABLE: Pubkey =
    pubkey!("E1jD3vdypYukwy9SWgWCnAJEvKC4Uj7MEc3c4S2LogD9");
  pub const STABILITY_POOL_LOOKUP_TABLE: Pubkey =
    pubkey!("Gb35n7SYMZCwCZbmxJMqoFsFX1mVhdSXmwo8wAJ8whWC");
  pub const REFERENCE_WALLET: Pubkey =
    pubkey!("GUX587fnbnZmqmq2hnav8r6siLczKS8wrp9QZRhuWeai");
}

#[cfg(feature = "shadow")]
mod deployment {
  use anchor_client::solana_sdk::{pubkey, pubkey::Pubkey};

  pub const EXCHANGE_LOOKUP_TABLE: Pubkey =
    pubkey!("<SHADOW_EXCHANGE_LUT>");
  pub const STABILITY_POOL_LOOKUP_TABLE: Pubkey =
    pubkey!("<SHADOW_STABILITY_POOL_LUT>");
  pub const REFERENCE_WALLET: Pubkey =
    pubkey!("<SHADOW_REFERENCE_WALLET>");
}

pub use deployment::*;
```

### 5. No changes needed

These all resolve correctly automatically:

- **PDAs** (`hylo-idl/src/pda.rs`): `LazyLock` calls
  `find_program_address` against `exchange::ID` / `stability_pool::ID`
  which are already swapped by step 2.
- **LST registry**: Fetched dynamically from on-chain Hylo account via
  `lst_registry_address()` in `program_client.rs`.
- **Pyth feed**: Same oracle regardless of deployment.
- **JITOSOL / HYLOSOL mints**: External tokens, unchanged.

## Usage

```bash
# Main deployment (default)
cargo build
cargo test

# Shadow deployment
cargo build --features shadow
cargo test --features shadow
```

## Verification

After implementing, confirm:
1. `cargo build` compiles with main values
2. `cargo build --features shadow` compiles with shadow values
3. `cargo test` passes (main)
4. `cargo test --features shadow` passes (shadow)
5. `nix develop .#nightly --command ./bin/lint.sh` clean on both
