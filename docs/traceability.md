# QASH Spec → Code → Test → Proof Traceability
> Normative source: `spec/pdf/QASH_Spec_v1.0.pdf`

## P0 — Genesis-Lock Prerequisites (current)
| ID | PDF § (p.) | PDF quote | Code | Test/Vector | Proof | Status | Blocking |
|---|---|---|---|---|---|---|---|
| P0-1 | §4.1 (pp. 9–10) | `L = W_D·D + W_C·C + W_S·Σ` / halt predicate | `crates/consensus/src/{lyapunov,transition}.rs` | — | `proofs/lyapunov_decrease.v` (TBD CI) | ⚠️ | Needs vectors + proof CI; ERR-001 resolved |
| P0-2 | §4.2 (p. 10) | `compute_state_root(state, crypto_suite)` | `crates/consensus/src/encoding.rs` | — | — | 🔶 | ADR-003 (PDF-SILENT) |
| P0-3 | §2.4 (pp. 4–5) | FixedPoint i128 SCALE=1_000_000 | `crates/consensus/src/fixed_point.rs` | `crates/consensus/src/fixed_point.rs` tests | — | ⚠️ | Add golden vectors |
| P0-4 | §2.5 (p. 5), §8.4 (pp. 23–24) | cross-ISA script in YAML | `.github/workflows/ci.yml` | `scripts/verify_cross_isa_identity.sh` + vectors | — | ❌ | script+runner missing |
| P0-5 | §2.3 (pp. 3–4) | `trigger_absorbing_halt(..)->!` | `crates/consensus/src/transition.rs` + `crates/pal` | — | — | 🔶 | ADR-004 |
| P0-6 | §3.2 (pp. 6–7) | compute_leaf_index concat | `crates/consensus/src/encoding.rs` | — | `proofs/concat_injective.v` (TBD CI) | ⚠️ | Add vectors + proof CI |
| P0-7 | §9.3 (p. 25) | `cd proofs && make all` + apalache | — | — | — | ❌ | Add Makefile/_CoqProject + TLA+ |
| P0-8 | §8.1 (p. 23) | Rust pinned | `rust-toolchain.toml` | — | — | ❌ | Add toolchain file |
| P0-9 | §2.1 (pp. 2–3), App E (p. 31) | genesis_hash computed | `GENESIS_CONSTANTS.toml` | vectors + sha3 tool step | — | ❌ | blocked by ADR-003 + ADR-001 threshold |
