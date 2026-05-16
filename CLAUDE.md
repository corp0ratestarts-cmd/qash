# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
# Build
cargo build
cargo build --release

# Test (CI uses --no-default-features)
cargo test --no-default-features
cargo test -p qash-consensus   # test a specific crate

# Check without building
cargo check

# Cross-compile (used for determinism verification)
cargo build --target aarch64-unknown-linux-gnu --release
cargo build --target riscv64gc-unknown-linux-gnu --release

# Verify deterministic build pipeline
./scripts/verify_two_stage_build.sh
```

## Architecture

QASH is a post-quantum, zero-governance, deterministic consensus protocol implemented as a Cargo workspace.

### Workspace layout

- **`crates/consensus`** (`qash-consensus`) — `no_std`, no-alloc consensus core. The only logic that is proof-eligible and replay-invariant. It includes substantive transition, encoding, fixed-point, hashing, Lyapunov, and parameter-fingerprint modules; all code here must satisfy the Domain A constraints below.
- **`crates/pal`** (`qash-pal`) — Platform Abstraction Layer. Defines `Time`, `Net`, `Attest`, and `Halt` traits. The `std` feature gates a `hosted::Host` stub implementation. PAL code is Domain B and may use `unsafe` under audit.
- **`src/`** — Hosted binary (`qash`) plus stub modules (`consensus`, `crypto`, `hardware`, `obfuscation`, `offline`). Most module files are currently empty stubs. `main.rs` is a thin entrypoint that calls `qash_consensus::consensus_hash`.
- **`GENESIS_CONSTANTS.toml`** — Immutable genesis parameters (fixed-point scale, Lyapunov weights, epoch timing, crypto cascade, hardware attestation modes, clone-protocol settings). Modifying this file defines a new network — treat it as append-only.
- **`proofs/`** — Coq/formal proofs: `contractivity/lyapunov_stability.v`, `safety/absorbing_halt.v`, and `cascade/` (TH-9, TH-10, TH-11 targets).

### Domain A vs Domain B (critical partition)

The protocol enforces a hard boundary between two execution domains defined in `00_execution_model.md`:

- **Domain A (Deterministic Consensus):** Everything in `qash-consensus` plus any future state-transition logic. Rules: no `unsafe`, no `f32`/`f64`, no `usize`/`isize`, no `HashMap` without deterministic seed, no wall-clock or entropy ingress, all arithmetic checked (overflow → absorbing halt). Must be replay-invariant across all authorized ISAs.
- **Domain B (PAL / Operational):** Everything in `qash-pal` and the hosted binary. Nondeterminism and `unsafe` are permitted here, but **Domain B values must never flow into Domain A computations**.

Cross-domain contamination (a Domain B value influencing a Domain A computation) is a protocol violation even if tests pass.

### Key protocol constants (from `GENESIS_CONSTANTS.toml`)

- Fixed-point scale: `1_000_000` (all scalar values are elements of `𝔽_p`)
- Intermediate arithmetic width: `i128`
- Overflow policy: `absorbing_halt` (irreversible halt, never panic or saturating arithmetic)
- Epoch duration: 500 ms, max control-loop latency 450 ms
- Post-quantum crypto cascade: Dilithium5 (primary), SLH-DSA-SHA3-256 (anchor), Falcon-512 (fallback)
- Consensus state root: folded SHA3-256 + SM3-256 primitive roots; obfuscation cascade: SHA3-256 → BLAKE3 → KangarooTwelve
- Max validators: 1024

### Arithmetic rules for Domain A code

- Only `u8`/`u16`/`u32`/`u64`/`u128`, `i8`/`i16`/`i32`/`i64`/`i128`, `bool` (as `u8 0x00/0x01`)
- `usize`/`isize` are forbidden in: consensus state struct fields, wire-format arithmetic, persisted values, and any value that crosses the Domain A/B boundary. `usize` IS permitted for array/slice indexing, loop iteration bounds, and buffer position tracking that is local to a single pure function. Rationale: Rust requires `usize` for indexing; this is unavoidable. The prohibition targets non-determinism from platform-dependent WIDTH used in ARITHMETIC or STATE.
- All arithmetic must be checked; overflow triggers `Halt::absorbing_reset()`
- No floating point anywhere in the consensus path
- Use `BTreeMap` not `HashMap` for deterministic iteration order
