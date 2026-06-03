# CLAUDE.md — Pure QASH

This file provides guidance to Claude Code when working in this repository.

## Identity

Pure QASH is a separate, privacy-maximal repository. It is NOT a feature flag or
profile variant of the umbrella `corp0ratestarts-cmd/qash` repo.
See `ADR-015` in the umbrella repo and `docs/adr/ADR-001-pure-qash-identity.md` here.

## Commands

```sh
# Build
cargo build
cargo build --release

# Test (always use --workspace)
cargo test --workspace --no-default-features
cargo test -p qash-consensus --no-default-features
cargo test -p qash-pal --features pure-qash

# Check without building
cargo check

# Cross-compile (determinism verification)
cargo build --target aarch64-unknown-linux-gnu --release
cargo build --target riscv64gc-unknown-linux-gnu --release

# Absence guard (must pass before any PR merges)
./scripts/check_pure_absence_guards.sh

# Evidence capture
cargo xtask capture-evidence
```

## Architecture

Pure QASH is a `no_std`, minimal, auditable implementation of:

```
Pure QASH =
  graph-non-publishing protocol
  + zero-persistence production profile
  + QASH Constitutional Scarcity Axiom
  + MEV-null Domain A economic surface
  + no regulatory disclosure key
  + no user graph evidence retention
  + no monetary governance
```

### Domain A vs Domain B (same partition as umbrella)

- **Domain A** (`crates/consensus/`): `no_std`, no-alloc, no floats, no `usize` in state fields,
  all arithmetic checked, overflow → `Halt::absorbing_reset()`. Replay-invariant.
- **Domain B** (`crates/pal/`): Platform abstraction. `unsafe` permitted under audit.
  Domain B values must never flow into Domain A computations.

### Key Pure QASH constraints (additional to Domain A rules)

- No Class IV observer class
- No genesis-authorised disclosure key
- No `lawful_basis` / `disclosure_domain` / `regulated_disclosure` anywhere
- No priority fees, no base-fee/tip splits, no mempool ordering
- No validator fee revenue
- No monetary governance (no oracle, rebase, discretionary treasury)
- No raw transaction retention in any production path
- No peer IP or routing metadata in any durable store
- `EphemeralEnvelope` must not implement `Serialize`, `Clone`, `Copy`, `Debug`, or `Display`

### Forbidden concepts — absence guards fail CI if these appear

```
ClassIV | class_iv | class-iv
lawful_basis | LawfulBasis
regulated_disclosure | RegulatedDisclosure
disclosure_key | DisclosureKey
priority_fee | PriorityFee
tip | base_fee_plus_tip
mempool
raw_tx_wal | RawTxWal
receipt_plaintext
peer_ip | socket_addr (in production paths)
impl Serialize for EphemeralEnvelope
impl Debug for EphemeralEnvelope
```

## Import Policy

Any code imported from `corp0ratestarts-cmd/qash` (umbrella) must:
1. Pass the absence guards above
2. Be recorded in `docs/release/import_manifest.md` with source commit SHA

Pure QASH does NOT automatically track umbrella QASH. Every sync is an explicit PR.

## PR Policy

- All PRs must pass `./scripts/check_pure_absence_guards.sh`
- Genesis constant changes require `[pure-qash-genesis-change-acknowledged]` in PR body
- Genesis-candidate changes require `[pure-qash-genesis-candidate-acknowledged]` in PR body
- No `v1.0-reference` tag without explicit owner genesis-candidate decision

## Genesis Status

```toml
genesis_status = "provisional"
deployment_authoritative = false
```

The current constants are provisional. Do not treat any value as network-canonical
until a genesis-candidate PR is merged with the required acknowledgement.
