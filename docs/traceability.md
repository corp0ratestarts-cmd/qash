# QASH Spec → Code → Test → Proof Traceability
> Normative source: `spec/pdf/QASH_Spec_v1.0.pdf`

## Genesis-Lock P0 Gate Summary (2026-05-14)

| Gate | Description | Status | Blocking |
|---|---|---|---|
| P0-1 | Cross-ISA determinism | 🟡 | x86_64 runner done + 5 golden vectors; QEMU/CI not yet triggered on aarch64/riscv64 |
| P0-2 | Coq CI integration | 🟡 | Coq job wired in CI; cascade_health_bounded.v proofs closed (0 Admitted in gated scope); needs CI run to confirm |
| P0-3 | `phi_max_safe` genesis pinning | ✅ | Pinned in GENESIS_CONSTANTS.toml + compile-time assert in params.rs |
| P0-4 | Nonzero slash-sum vectors | ✅ | 3 categories present: below (511v), at-boundary=violation (512v), lyapunov halt |

## P0 Prerequisite Table

| ID | PDF § (p.) | PDF quote | Code | Test/Vector | Proof | Status | Blocking |
|---|---|---|---|---|---|---|---|
| P0-1 | §4.1 (pp. 9–10) | `L = W_D·D + W_C·C + W_S·Σ` / halt predicate | `crates/consensus/src/{lyapunov,transition}.rs` | golden_replay.rs (5 unit tests) + vectors.v1.json (5 vectors) | `proofs/contractivity/lyapunov_stability.v` TH-3a/3b/3c (0 Admitted) | 🟡 | Cross-ISA CI run pending QEMU install on runner |
| P0-2 | §4.2 (p. 10) | `compute_state_root(state, crypto_suite)` | `crates/consensus/src/{encoding,transition}.rs` (ADR-003 Accepted) | vectors.v1.json — 5 golden `state_root_hex` values | — | 🟡 | Roundtrip + rejection tests pending |
| P0-3 | §2.4 (pp. 4–5) | FixedPoint i128 SCALE=1_000_000 | `crates/consensus/src/fixed_point.rs` | fixed_point.rs unit tests | — | 🟡 | Add golden vectors for fixed_point operations |
| P0-4 | §2.5 (p. 5), §8.4 (pp. 23–24) | cross-ISA script in YAML | `.github/workflows/ci.yml` (cross-isa job) + `crates/vector-runner` (implemented) | `scripts/verify_cross_isa_identity.sh` + `tests/vectors/vectors.v1.json` | — | 🟡 | CI aarch64/riscv64 runs pending; `continue-on-error: true` until runners provisioned |
| P0-5 | §2.3 (pp. 3–4) | `trigger_absorbing_halt(..)->!` | `crates/consensus/src/transition.rs` (HaltReason 7 variants) + `crates/pal` (stub) | golden_replay.rs halt tests | — | 🔶 | ADR-004 Proposed; PAL zeroize/watchdog not implemented |
| P0-6 | §3.2 (pp. 6–7) | compute_leaf_index concat | `crates/consensus/src/encoding.rs` | — | `proofs/concat_injective.v` (TBD) | ⚠️ | Add vectors + proof |
| P0-7 | §9.3 (p. 25) | `cd proofs && make all` + apalache | `proofs/Makefile` + `.github/workflows/ci.yml` (coq-proofs job) | CI coq-proofs job | TH-9 proved; TH-10/TH-11 axioms | 🟡 | Apalache TLA+ job still echo-only; needs install step |
| P0-8 | §8.1 (p. 23) | Rust pinned | `rust-toolchain.toml` (1.75.0) | — | — | ✅ | Done |
| P0-9 | §2.1 (pp. 2–3), App E (p. 31) | genesis_hash computed | `GENESIS_CONSTANTS.toml` (`phi_max_safe` pinned) | — | — | ❌ | `genesis_hash` still PLACEHOLDER; blocked by finalization of all P0 gates |

## ADR Status
| ADR | Title | Status |
|---|---|---|
| ERR-001 | Lyapunov two-function partition | Accepted |
| ADR-001 | Φ_safety aggregation + threshold + halt gate | Accepted |
| ADR-003 | `compute_state_root` encoding + commitment | Accepted |
| ADR-004 | Absorbing halt layering: consensus vs PAL | Proposed |
