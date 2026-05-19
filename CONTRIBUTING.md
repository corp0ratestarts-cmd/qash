# Contributing to QASH

## Prerequisites

```sh
# Rust (stable toolchain, pinned via rust-toolchain.toml when present)
rustup toolchain install stable
rustup target add aarch64-unknown-linux-gnu riscv64gc-unknown-linux-gnu

# Coq (for proof work)
apt-get install coq        # Debian/Ubuntu
brew install coq           # macOS

# cargo-deny (supply chain)
cargo install cargo-deny

# Optional: cross-compilation
cargo install cross
```

## Building and testing

```sh
# Build
cargo build --no-default-features

# Full test suite (what CI runs)
cargo test --workspace --no-default-features

# Single crate
cargo test -p qash-consensus --no-default-features

# Lint (must be clean — CI enforces -D warnings)
cargo clippy -p qash-consensus -- -D clippy::unwrap_used -D clippy::expect_used \
    -D clippy::panic -D clippy::unreachable -D warnings

# Supply chain
cargo deny check

# Coq proofs
cd proofs
coqc -Q . QASH util/list_inj.v
coqc -Q . QASH contractivity/lyapunov_stability.v
for f in concat_injective.v contractivity/encode_injectivity.v \
    contractivity/tx_perturbation_0.v contractivity/tx1_score_decrement.v \
    contractivity/lyapunov_grace_convergence.v safety/absorbing_halt.v \
    integration/th8_composition.v cascade/cascade_health_bounded.v \
    cascade/cascade_determinism.v cascade/cascade_collision_resistance.v \
    blinding/blinding_non_interference.v model/Model.v; do
  coqc -Q . QASH "$f"
done
```

## Domain partition — what goes where

QASH enforces a hard boundary between two execution domains:

### Domain A — Deterministic Consensus (`crates/consensus/`)

Rules that **must never be violated**:

| Rule | Rationale |
|------|-----------|
| No `unsafe` | Prevents memory safety violations in consensus-critical code |
| No `f32`/`f64` | Floating-point is non-deterministic across ISAs |
| No `usize`/`isize` in struct fields, wire arithmetic, or persisted values | Width is platform-dependent; use `u32`/`u64` instead |
| `usize` is permitted for array indexing and loop bounds | Rust requires it; the prohibition targets arithmetic/state |
| No `HashMap` without a deterministic seed | Iteration order varies; use `BTreeMap` |
| No wall-clock, entropy, or I/O ingress | All nondeterminism routes through Domain B |
| All arithmetic must be checked; overflow → `Halt::absorbing_reset()` | Silent wrap or panic is a safety violation |
| No `unwrap()`/`expect()`/`panic!()`/`unreachable!()` | Same reason — panic is not absorbing halt |

### Domain B — PAL / Hosted binary (`crates/pal/`, `src/`)

- `unsafe` is permitted under audit
- Nondeterminism is permitted
- **Domain B values must never flow into Domain A computations** — this is a protocol violation even if all tests pass

## PR checklist

The `.github/PULL_REQUEST_TEMPLATE.md` contains the full checklist. Key gates:

1. **Deduplication** — search open and recently closed PRs before opening
2. **Human review** — a human (not only an AI assistant) must read and understand the changes
3. **Domain A constraints** — verify all rules above if touching `crates/consensus/`
4. **Coq proofs** — if adding/changing `.v` files, zero `Admitted` markers allowed outside `_wip/`; add to CI Tier 2 list in `ci.yml`; add row to `proofs/COVERAGE.md`
5. **Genesis constants** — any change to `GENESIS_CONSTANTS.toml` requires `[genesis-change-acknowledged]` token in the PR body; this defines a new network

## Adding a new hash primitive to the cascade

The 8-family cascade in `crates/consensus/src/derive.rs` follows a strict pattern:

1. Add the crate to `crates/consensus/Cargo.toml` with `default-features = false`
2. Add a `path_X_<name>` function with a unique domain separator `b"QASH:DERIVE:X:<NAME>:<STANDARD>"`
3. If the new crate uses a different version of `digest` than existing deps, scope the trait import inside the function: `use new_crate::digest::Digest as _;`
4. Update `derive_leaf_index`: add the new path, widen `all_paths`, update the forgery bound comment
5. Update the module doc table and security argument
6. Add `path_X_<name>_deterministic` test and extend `all_paths_are_distinct`
7. Update `proofs/COVERAGE.md` IT-MAC forgery bound row

## Adding a new Coq proof

1. Place the file in the appropriate subdirectory (`contractivity/`, `safety/`, `cascade/`, etc.)
2. Do not use `Admitted` — use `Axiom` with a comment if deferring
3. Add to the Tier 2 compile list in `.github/workflows/ci.yml`
4. Add a row to `proofs/COVERAGE.md`
5. Update `proofs/STATUS.md`

## Commit style

- One logical change per commit
- Present tense, imperative: `add`, `fix`, `update`, not `added`, `fixed`
- Reference the spec section if the change implements a protocol property: `lyapunov: implement TH-3a halt gate (§4a)`


## Crypto dependency bump gate

Any crypto dependency bump requires a refreshed conformance artifact from the `crypto-conformance` CI job. See `docs/compliance/crypto_dependency_policy.md`.
