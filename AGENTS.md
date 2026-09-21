# AGENTS.md

Guidance for coding agents working in this repository.

## Overview

Rust SDK for the Hylo Protocol — a collateralized stablecoin and leverage
token DEX on Solana. Six published crates wrapping three onchain programs.
The programs themselves live in `~/Github/protocol`, not here.

Workspace version is shared: `[workspace.package].version` in the root
`Cargo.toml`, currently mirrored by all six path dependencies.

---

## Environment

Nix flake with three devshells. `.envrc` is `use flake .`, so direnv puts you
in `default` on entry — run `cargo`, `build`, `polish`, `lint` bare, never
wrapped in `nix develop --command`. Only `.#nightly` and `.#kani` need
explicit entry, and `polish` / `lint` enter `.#nightly` themselves.

| Shell | Contents | Used for |
|-------|----------|----------|
| `nix develop` | Rust stable 1.88.0 (+ `rust-analyzer`, `rust-src`), `evcxr`, `cargo-semver-checks` | build, test, publish |
| `nix develop .#nightly` | nightly, `cargo-udeps` | `lint`, `polish`, `cargo +nightly udeps` |
| `nix develop .#kani` | `rustup`, `cmake` | `cargo kani` proofs |

Shared `buildInputs`: `libiconv`, `pkg-config`, `gcc`, `openssl`.

### Commands

Defined in `shell-tools.nix`, exposed as flake `packages`. On `PATH` inside
any devshell; also `nix run .#<name>`.

**After completing any coding task, always run:**

```bash
polish   # Format + auto-fix clippy
lint     # Verify in CI check mode
```

| Tool | What it runs |
|------|--------------|
| `polish` | In `.#nightly`: `cargo fmt`; `cargo clippy --fix --all-targets --all-features`; `cargo clippy --fix --all-targets -p hylo-core --no-default-features` |
| `lint` | In `.#nightly`: the same three with `--check` |
| `build` | `cargo build` |
| `test-cargo` | `cargo test --workspace --exclude hylo-jupiter`; the same again with `--features shadow`; then `cargo test --doc` |
| `verify` | In `.#kani`: `cargo kani "$@"` |
| `publish` | Polls crates.io per crate, skips any already at the workspace version, then `cargo build --release` + `cargo doc --workspace --no-deps` + `cargo publish` for the rest |

Both clippy passes matter. `hylo-core` must stay clean with default features
off, because that build is the onchain-compatible one.

`test-cargo` runs the workspace twice — once plain, once with `shadow`.

Narrower runs:

```bash
cargo test -p hylo-quotes
cargo +nightly udeps            # nightly shell
nix develop .#kani -c cargo kani -p hylo-core
```

Integration tests need environment variables. `RPC_WS_URL` may be fake.

```
RPC_URL=https://mainnet.helius-rpc.com/?api-key=<key>
RPC_WS_URL=wss://mainnet.helius-rpc.com/?api-key=<key>
```

Never carry a build, test, or lint command over from another project. The
flake is the authority; confirm an attribute with `nix flake show` rather
than guessing it.

---

## Crates

Dependency flow:

```
hylo-core (no hylo deps) → hylo-idl → hylo-clients → hylo-quotes → hylo-jupiter
                                    → hylo-stats
```

`hylo-stats` sits outside the client chain: it depends only on `hylo-core`
and `hylo-idl`, and fetches accounts with a raw `RpcClient`.

### `hylo-core`

Pure protocol math and types. No RPC.

`exchange_math`, `earn_pool_math`, `collateral_ratio`, `borrow_rate`,
`conversion`, `calculus`, `par_tolerance`, `virtual_stablecoin`, `yields`,
`solana_clock`, `asset_swap_config`, `slippage_config`, `error`, plus the
submodules `fees/` (controller, curve_controller, curves, interp),
`limiter/` (deposit, withdraw, levercoin), `rebalance/` (math, mode, pnl,
pricing, pool_drawdown), `lst/` (stake_pool, sol_price, total_sol_cache),
`pyth/` (feeds, oracle), `exchange_context/` (lst, exo, marginal).

The `offchain` feature enables `hylo-idl`, the Jupiter AMM interface, and
`hylo-fix/typed-floats`, and gates `calculus` and `idl_type_bridge`. All
downstream crates depend on `hylo-core` with `offchain` on.

### `hylo-idl`

Anchor IDL codegen and type-safe token definitions. No math, no RPC.

`declare_program!` for `hylo_exchange`, `hylo_earn_pool`, `hylo_router`
(and their shadow variants). `tokens` (`TokenMint`, `StakePool`, `Exo`,
`with_exo_pairs!`), `pda` (`pda!`, `pda_with_bump!`), `account_builders/`,
`instruction_builders/`, `type_bridge`, and the `lut-accounts` binary.

### `hylo-clients`

Transaction builders and execution clients.

`RouterClient` handles every user-facing operation — mint, redeem, swap,
earn pool deposit and withdraw. `ExchangeClient` and `EarnPoolClient` are
admin only. Also `ProgramClient`, `BuildTransactionData`,
`TransactionSyntax`, `InstructionBuilder`, `squads` multisig, `memo`,
`util::LST`, and `prelude`.

### `hylo-quotes`

Quoting strategies over protocol state or simulation.

`ProtocolStateStrategy`, `SimulationStrategy`, `RuntimeQuoteStrategy`,
`TokenOperation` / `TokenOperationExt`, `SimulatedOperation`,
`StateProvider` / `RpcStateProvider`, `protocol_state/` including
`exo_registry` and `exo_pair`, `QuoteMetadata` / `Operation`,
`ExecutableQuote` and `ExecutableQuoteValue`.

### `hylo-jupiter`

Jupiter AMM integration. `HyloJupiterPair`, `HyloJupiterExo`, `PairConfig`,
`account_metas`. Excluded from workspace tests.

### `hylo-stats`

Offchain earn pool yield statistics. `client` (raw `RpcClient`, no keypair),
`earn_pool_stats`, `earn_pool_yield_math`, `types`, `error`.

### Quality tiers

`hylo-core`, `hylo-clients`, and `hylo-idl` are held to exacting correctness
standards. `hylo-quotes` and `hylo-stats` are lower-trust legacy code —
still fix what is wrong, but do not assume existing patterns there are
endorsed.

---

## Features

### `offchain` (`hylo-core` only)

Enables `hylo-idl`, `hylo-jupiter-amm-interface`, `hylo-fix/typed-floats`.
Gates `hylo_core::calculus` and `hylo_core::idl_type_bridge`, and re-exports
`hylo_idl` as `hylo_core::idl`.

### `shadow` (every crate)

Swaps each `declare_program!` to the `*_shadow` IDL and its separate program
IDs — a parallel staging deployment. Propagates
`hylo-idl` → `hylo-core` → `hylo-clients` → `hylo-quotes` → `hylo-jupiter`,
and `hylo-stats`.

| Program | Mainnet | Shadow |
|---------|---------|--------|
| Exchange | `HYEXCHtHkBagdStcJCp3xbbb9B7sdMdWXFNj6mdsG4hn` | `hyshEX5sNEYhnYPMm8MwMThhBRPuLN3rjoYDbC9esPQ` |
| Earn pool | `HysTabVUfmQBFcmzu1ctRd1Y1fxd66RBpboy1bmtDSQQ` | `HYShEAST5PHe5EFxUPYUgzXsmSo88VVdDqJE21jXBQ7N` |
| Router | `hyRouTRDAgn65xyyJ3L5c4k5SFmSdr3NxDV8Euzjy3f` | `HyshRo2hkqXGcyCfKU22zhSBPMwokmAnEoxDGeVQz7d` |

These IDs are asserted by tests in `hylo-idl/src/lib.rs`. Any change to a
program ID must keep both branches of those tests passing.

---

## Rust style

Write like a functional programmer. Types are the specification; the body
should read as a derivation from them.

### Shape

* Model the domain in types first. Make illegal states unrepresentable —
  newtypes over bare primitives, enums over a `bool` plus a comment,
  `NonZeroU64` / `Option` over sentinel values.
* Parse, don't validate. Convert unstructured input to a typed value once,
  at the boundary. Downstream code receives the typed value and cannot
  re-check it.
* Total functions. If it can fail it returns `Result`; if it cannot, the
  signature says so.
* Small single-purpose functions composed together, not long procedures.
  Extract the named concept, not merely the repeated lines.
* Abstract on the third occurrence, not the first. An abstraction must
  remove a concept, not relocate code.
* Exhaust function composition before reaching for a wrapper struct, a
  generic trait, or a macro change. This repo is macro-heavy; adding an arm
  to an existing macro is cheaper than adding a layer above it.
* Keep semantically distinct helpers separate even when their bodies look
  duplicated. A helper that carries its own domain name earns its existence.
* Don't add a public method with no current caller, however plausible it
  looks as an SDK convenience.

### Expressions

* Everything is an expression: `let x = if ... { } else { }`,
  `let y = match ... { }`.
* Never use `return`. The tail expression is the value. Early exit via `?`,
  or restructure the function.
* Never use `for` or `while` loops. Iterators and combinators — `map`,
  `filter`, `filter_map`, `fold`, `try_fold`, `collect`, `zip`, `partition`,
  `sum`.
* Combinators over matching on `Option` / `Result` — `map`, `and_then`,
  `unwrap_or_else`, `ok_or`, `transpose`. Match only when the arms are
  genuinely distinct logic.
* No mutable accumulator where `fold` or `collect` works. Reaching for `mut`
  is a claim that no combinator fits; that claim is usually false.
* Return `impl Iterator` rather than a materialized `Vec` when the caller
  may not need every element.
* Prefer the explicit type name over `Self` in signatures and constructors.
  A preference, not a hard rule.

### Errors

* `?` for propagation. Never `.unwrap()` or `.expect()` in library code.
* Never `.map_err(Into::into)` — `?` already applies the `From` impl.
* One error variant per distinct failure. No stringly-typed catch-all.
* Validation functions take the storage type, convert internally, and return
  it, so the caller inlines `self.field = validate_foo(value)?;`.
* Never construct throwaway state and mutate it purely to surface a
  validation error.
* Every default in math is a claim about what absence means, so check it
  against the intent of the computation before writing it. `unwrap_or_default()`
  and `unwrap_or(0)` are legitimate when zero is the domain's answer for the
  missing case — an absent cap yielding zero headroom — and wrong when the
  `None` came from an arithmetic failure, where a silent zero propagates
  instead of the error. Write the reason at the call site so the next reader
  scrutinizes the same claim rather than reconstructing it.
* Apply the same scrutiny to a derived `Default` on a type carrying a price,
  an amount, a ratio, or a rate. Zero is rarely a member of those domains.

Error types by crate:

| Crate | Type |
|-------|------|
| `hylo-core` | `CoreError` in `error.rs` — Anchor `#[error_code]`, discriminants based at 7000, grouped by source module |
| `hylo-stats` | `StatsError` in `error.rs` — `thiserror` |
| `hylo-clients`, `hylo-quotes`, `hylo-jupiter` | `anyhow::Result<T>` |

### Naming

* Domain-specific parameter names over generic ones — `spot_price`,
  `projected_price`, not `price`, `spot`, `oracle`, `endpoint`.
* No precision or exponent suffixes — `collateral_in`, not
  `collateral_n9`. The type carries the precision.
* For `&mut self` methods that fold a single event into running state,
  prefer `update_*` over `accumulate_*` or `record_*`.
* Modules and functions `snake_case`; structs and traits `PascalCase`;
  constants `SCREAMING_SNAKE_CASE`; generic type parameters single letters
  (`IN`, `OUT`, `C`, `L`).
* Write "exo" lowercase in prose, docs, and error messages. It is not an
  acronym. Type names keep their existing casing.

### Mechanics

`rustfmt.toml` is authoritative:

```
tab_spaces = 2
max_width = 80
imports_granularity = "Module"
group_imports = "StdExternalCrate"
format_strings = true
wrap_comments = true
comment_width = 80
```

* `&str` over `String`, `&[T]` over `&Vec<T>` in signatures when ownership
  isn't needed.
* Group imports: std, external crates, crate-internal — blank line between.
  All at the top of the file; never a `use` inside a function body.
* Module-level constants go directly after the imports, not beside the
  functions that use them.
* `use foo::Bar;` then `Bar` — never `foo::Bar` inline.
* Turbofish is all or nothing. Never name some type parameters and leave
  others as `_`.
* Derive order: `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize,
  Deserialize`.

Workspace lints (root `Cargo.toml`):

```toml
[workspace.lints.clippy]
pedantic = { level = "deny", priority = -1 }
needless_for_each = "allow"
type_complexity = "allow"
```

All pedantic lints must pass. `hylo-core/src/lib.rs` allows
`clippy::missing_errors_doc` and `clippy::wildcard_imports` crate-wide; the
`missing_errors_doc` allow is an omission to be retired, not a licence to
skip `# Errors` on new public code.

---

## Documentation

Documentation states what the code does. Nothing else. The audience is an
outside reader with zero session context.

* One sentence. Present tense, active voice, indicative mood. A second
  sentence only to state a constraint the signature cannot express. If it
  runs past one wrapped line it is too long.
* Verb phrase for functions, noun phrase for types. No subject: write
  "Converts a spot price to quote precision.", never "This function converts
  ...". Never restate the name — `validate_foo` does not get "Validates
  foo."
* Don't restate the signature. Names, types and `Result` are visible.
  Documentation adds only what the types cannot carry — units, invariants,
  ordering, side effects, precision.
* Never address the reader. No "you", "we", "our", "let's", "note that",
  "keep in mind", "make sure", "as expected".
* No quality adjectives — "efficient", "robust", "safe", "flexible",
  "powerful", "clean", "simple", "convenient", "seamless",
  "comprehensive".
* No filler verbs — "leverage", "utilize", "facilitate", "ensure",
  "handle", "manage", "perform", "provide". Name the actual operation.
* No hedging — "may", "might", "typically", "generally", "essentially",
  "effectively", "under the hood", "in practice".
* Don't invent words or compounds. Use terms that already appear in the code
  or the domain. A concept with no name in the code gets no name in a
  comment.
* Don't hyphenate to build modifiers. Rephrase: "the loop over open
  positions", not "the open-position iteration loop".
* No history and no process — "now uses", "previously", "new",
  "refactored", "as discussed", "see the plan".
* No examples unless the API is easy to misuse. No headings other than
  `# Errors`. No ASCII diagrams. No exclamation marks, rhetorical questions,
  or parenthetical asides.
* Use `*` for bullet lists in rustdoc, never `-`.
* Spell "onchain" and "offchain" unhyphenated, everywhere.
* Read two or three sibling items before writing one, and match their
  density. Read the line back; if it sounds like a chat reply, delete it and
  keep the one clause that carries information.

Write:

```rust
/// Converts a spot price to the vault's quote precision.
pub fn to_quote(spot_price: Price) -> Quote
```

Not:

```rust
/// This is a helper function that takes in a spot-price value and seamlessly
/// handles converting it into the vault's quote-precision representation,
/// ensuring correctness across all supported markets.
pub fn to_quote(spot_price: Price) -> Quote
```

### Scope

* Doc comments on public items. Private helpers stay undocumented.
* New structs get one struct-level doc, no per-field `///`. No `///` above a
  trait impl restating its type parameters.
* `# Errors` on every public item that returns a `Result`: short `*`
  bullets naming the failure conditions. Never name the error variants; they
  are visible in the code.
* Module `//!` preambles are neither required nor suggested. The crate-root
  preambles in `hylo-clients`, `hylo-quotes`, and `hylo-stats` carry
  CI-enforced doctests — keep them compiling. Don't add new ones.

### Inline comments

* Don't narrate or justify individual statements. Names carry the meaning.
* Section comments delineating the phases of a multi-step computation are
  useful — keep them. The rule targets justification noise, not signposts.
* Constants get the value and unit or nothing: `/// 1000 bps (10%)`.
* Non-trivial math goes in multi-line ASCII fraction form, not an inline
  expression:

  ```
  //          collateral * price
  // ratio = --------------------
  //          stablecoin_supply
  ```

* Neutral technical prose. No metaphors, no coined shorthand, no lingo
  carried over from a conversation.
* Never write chat, plan, or design-rationale context into a comment — no
  "as agreed", "mirrors X", "sets up the later refactor".
* Preserve the user's own wording verbatim when moving or extending their
  code. Don't improve their phrasing.

Commit messages, PR bodies, and docs follow the same rule: name the concrete
symbol, file, or behavior, and assume the reader has only the diff and the
repo.

---

## Protocol bindings and types

* Program bindings come from `anchor_lang::declare_program!` over the JSON
  IDLs in `hylo-idl/idls/`. Generated types are not edited by hand; an IDL
  change is a file replacement plus whatever the compiler then demands.
* Account and instruction construction lives in
  `hylo-idl/src/account_builders/` and `hylo-idl/src/instruction_builders/`,
  one module per program.
* PDAs are derived through the `pda!` and `pda_with_bump!` macros in
  `hylo-idl/src/pda.rs`. Never inline a `find_program_address` call at a use
  site.
* Validation is the program's job. The SDK's job is to make an invalid
  request unrepresentable in the type system before it is ever sent.
* Slippage is in basis points (50 = 0.5%).
* Default compute units: 100k with buffer (`DEFAULT_CUS_WITH_BUFFER`).

### Type-safe token system

`TokenMint` defines a token with an associated `Exp` for decimal precision
and a `const MINT: Pubkey`. Operations are generic over token pairs:
`<IN: TokenMint, OUT: TokenMint>`.

| Exponent | Tokens |
|----------|--------|
| `N6` | `HYUSD`, `SHYUSD`, `XSOL`, `USDC`, `PST`, and every levercoin (`XBTC`, `XZEC`, `XONYC`, `XHYPE`, `XPST`, `XETH`) |
| `N8` | `CBBTC`, `ZEC`, `WETH` |
| `N9` | `JITOSOL`, `HYLOSOL`, `HYPE`, `ONYC` |

Marker traits: `StakePool` (LSTs, `TokenMint<Exp = N9>`), `Exo` (exogenous
collateral), `hylo_clients::util::LST`.

### Exo pairs are generated

`with_exo_pairs!` in `hylo-idl/src/tokens.rs` is the single list of exo
collateral and its levercoin:

```
(CBBTC, XBTC, N8), (HYPE, XHYPE, N9), (ONYC, XONYC, N9),
(PST,   XPST, N6), (WETH, XETH,  N8), (ZEC,  XZEC,  N8)
```

It is a callback macro — it invokes a "shaper" macro once per pair. Roughly
a dozen shapers expand from it across the workspace: `impl_exo`,
`impl_local_exo`, `exo_pair_dispatch`, `exo_state_quotes`,
`exo_simulation_quotes`, `exo_router_instructions`,
`exo_router_transaction_data`, `exo_levercoin_ops`, `exo_simulated_ops`,
`exo_pyth_feed_dispatch`.

**Adding an exo pair means editing one macro, not a dozen call sites.** If a
change requires touching every shaper individually, the change is in the
wrong place.

### Fixed-point math

`hylo-fix` 0.8.0 from crates.io, imported under the lib name `fix`.

```rust
use fix::prelude::*;
```

`UFix64<Exp>` carries compile-time decimal precision as a type parameter
(`N6`, `N9`). `UFixValue64` is its runtime-exponent counterpart.

* Never drop to `.bits` or raw `u64` arithmetic to work around a type
  mismatch, and never fabricate a `one()` or an exponent constant. Use the
  `fix` operations.
* Put an inverse operation directly beside its forward operation.
* `.bits` is for raw access at a boundary, not for doing math.

### Quoting

`RuntimeQuoteStrategy` dispatches from runtime `Pubkey`s to compile-time
typed `QuoteStrategy<IN, OUT, C>` implementations. The route table in
`hylo-quotes/src/runtime_quote_strategy.rs` currently holds 68 routes: the
LST and USDC core routes, earn pool deposit and withdraw, and six exo
families of eight routes each (mint, redeem, levercoin mint, levercoin
redeem, two conversions, two USDC swaps).

Two strategies:

* `ProtocolStateStrategy` — quotes from cached protocol state. Fast, no
  wallet balance check.
* `SimulationStrategy` — quotes by simulating the transaction. Slower,
  validates that the transaction would actually succeed.

### Common patterns

Building a transaction:

```rust
use hylo_clients::prelude::*;
use hylo_idl::tokens::{HYUSD, JITOSOL};

let client = RouterClient::new_random_keypair(
  Cluster::Mainnet,
  CommitmentConfig::confirmed(),
)?;

let signature = client.run_transaction::<JITOSOL, HYUSD>(
  RouterArgs { amount: 1_000_000_000, user, slippage_config: None },
).await?;
```

Computing a quote from protocol state:

```rust
let op = state.output::<JITOSOL, HYUSD>(amount_in)?;
```

Preludes: `hylo_clients::prelude::*` for clients and tokens,
`hylo_quotes::prelude::*` for quoting, `fix::prelude::*` for math.

---

## Testing

* `hylo-core`: unit tests in-module. `proptest` for property tests,
  `more-asserts`, and the `eq_tolerance!` macro in `util.rs`.
* Kani proofs live in `#[cfg(kani)] mod` blocks beside the code they prove —
  `exchange_math`, `conversion`, `fees/{interp,controller}`,
  `rebalance/{pnl,mode,math}`. Generators in `hylo-core/src/kani_generators.rs`,
  also `cfg(kani)`. Run with `verify` or
  `nix develop .#kani -c cargo kani -p hylo-core`. `cfg(kani)` code is
  invisible to `cargo test`, `polish`, and `lint` — a proof can rot silently.
* `hylo-quotes/tests/state_based_tests.rs` smoke-quotes every route against a
  mainnet account snapshot in `tests/data/`. Prices move between snapshots,
  so these assert that a route produces output, not what it produces.
  `dump_protocol_accounts()` writes a fresh snapshot.
* `hylo-quotes/tests/integration_tests.rs` compares `ProtocolStateStrategy`
  against `SimulationStrategy` under
  `#[test_context(QuoteStrategyTestContext)]`. Needs `RPC_URL`; uses
  `hylo_clients::util::REFERENCE_WALLET`.
* Doctests are CI-enforced via `cargo test --doc`. The `rust,no_run`
  examples in the `hylo-clients` and `hylo-quotes` crate roots must keep
  compiling.
* Do not use the `flaky_test` macro. It was removed from the codebase.
* `hylo-jupiter` is excluded from workspace tests.

---

## CI and versioning

`.github/workflows/ci.yml`, three chained jobs:

1. **`check-sdk-version`** (pull requests only) — `bin/check-sdk-version.sh`
   requires the workspace version be exactly one patch, minor, or major bump
   above `main`. Anything else fails the PR.
2. **`build`** — Nix installer, magic-nix-cache, sccache, then
   `nix run .#lint` → `nix run .#build` → `nix run .#test-cargo`.
3. **`publish`** — on push to `main` only, `nix run .#publish`.

Versioning is loose. Breaking API changes ship in minor bumps; a major bump
is not required for them.

Any SDK API change requires grepping every `hylo-so` consumer repository for
call sites before it ships. A change that compiles here can still break a
downstream repo that this workspace does not build.

---

## Working agreements

### Repository boundaries

* **`~/Github/protocol`** — never edit. Audited onchain code; changes are
  applied by the repository owner personally. Read it freely.
* **`~/Github/fix`** — never edit. `hylo-fix` is consumed as a pinned
  crates.io dependency. Supply a diff or snippet for the owner to apply.
* **This repository** is public (`hylo-so/sdk`). See Disclosure.

### Scope

Confirm the interpretation before acting when any of these hold:

* The request admits two readings that lead to materially different work.
* Scope is unbounded — "clean up", "fix the tests", "make it better" with no
  named target.
* The change is wide — new module, cross-file refactor, dependency change,
  public API change — and wasn't spelled out.
* Executing would discard work: force-push, history rewrite, file deletion,
  overwriting uncommitted changes.

State the interpretation in one line and ask. Do not open with a plan built
on a guess.

Act without asking when the target is named and the change is bounded — a
specific file, function, error, or a literal instruction. Do not ask
permission to begin work already described.

Never widen scope past what was asked. Adjacent problems found along the way
get reported, not fixed.

### Edit discipline

One `Edit` or `Write` per assistant message. Never two or more in the same
tool-call block, including during review passes and follow-up fixes. Each
tool call is accepted or rejected individually; a batch renders as one blob
and gets rejected wholesale.

* Name the chunks up front, then emit one per message under a one-line
  header.
* One test is one chunk. One struct plus its impl is one chunk. Don't
  fragment trivial work — a few imports, a rename, a one-line fix.
* Keep going through the chunks without asking "ready for the next?".
* Never replace an entire existing file with one `Write` when edits would
  do.
* During a batch of related edits, defer `polish` / `lint` / `test` until
  the batch is done rather than running them after each one.

### Shell hygiene

* Long-form CLI flags always — `--recursive`, not `-r`. Where a tool has no
  long form, keep the short flag and say what it does.
* No `echo "=== label ==="` scaffolding stitching sub-commands together.
* Never make a structural source edit with `sed`, `awk`, or a heredoc
  rewrite — moving functions, splicing structs, deleting match arms. Use
  Read + Edit, or Write for a full-file rewrite, so the diff renders inline
  and stays reviewable.
* **Never run `nix build` inside this repository** — it drops a `result`
  symlink into the working tree. Use `nix run .#<target>`, plain `cargo`, or
  pass `--no-link`.

### Git

* Small, focused commits — one logical change each.
* Rebase feature branches onto `main` before merging.
* Do not commit until explicitly asked. Leave changes in the working tree
  for review.

### Disclosure

This repository is public.

* Never reference the security audit — issue numbers, the word "audit",
  auditor or firm names — in commit messages, PR titles or bodies, code
  comments, or branch names. Describe the change on its own technical terms.
  Linking a public commit to a finding discloses an unpatched vulnerability
  and its location.
* If an audit reference reaches the remote, amend and force-push
  immediately, and flag that the old SHA can persist in caches, forks, and
  GitHub event logs.
* Never add "Generated with Claude Code" attribution to commits or PR
  bodies.
* Never write credentials into memory files, plans, commits, or docs — API
  keys, RPC URLs with an embedded key, keypair paths, seed phrases. Refer to
  the environment variable or the secret's location instead.
