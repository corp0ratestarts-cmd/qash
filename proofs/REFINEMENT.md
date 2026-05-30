# QASH Proof-to-Code Refinement

Documents the three-layer chain from formal specification to Rust implementation,
and the empirical evidence layer that bridges them.

---

## 1. Three-Layer Architecture

```
Layer 1: Coq formal model (proofs/)
    ↓  extraction to
Layer 2: OCaml reference model (not committed; generated from Coq)
    ↓  behavioural equivalence under AX-2
Layer 3: Rust implementation (crates/consensus/)
    ↓  witnessed by
Layer 4: CI test vectors (tests/vectors/)
```

**AX-2 (Compiler Correctness):** States that the Coq-extracted OCaml model and the Rust
implementation are observationally equivalent — same inputs produce same outputs on all
authorized ISAs (x86_64, aarch64, riscv64gc). Supported by the multi-compiler differential CI
job (`multi-compiler-diff.yml`) and 10+ CI test vectors.

AX-2 does not claim:
- That the Rust code is formally extracted from Coq (it is hand-translated).
- That the extraction is checked by a tool (post-genesis enhancement).
- That any particular Coq tactic produces a specific Rust function body.

---

## 2. Theorem ↔ Rust Function Mapping

| Coq theorem | File | Rust function | Module | Status |
|-------------|------|---------------|--------|--------|
| `lyapunov_stability` | `proofs/contractivity/lyapunov_stability.v` | `advance_epoch` | `crates/consensus/src/transition.rs` | PROVED |
| `absorbing_halt_correct` | `proofs/safety/absorbing_halt.v` | `Halt::absorbing_reset()` (PAL), halt guard in `advance_epoch` | `crates/consensus/src/transition.rs`, `crates/pal/` | PROVED |
| `cascade_collision_resistance` | `proofs/cascade/cascade_collision_resistance.v` | `h_cascade()` | `crates/consensus/src/cascade.rs` | AXIOM (AX-3) |
| `transition_safe_fixed_point` | `proofs/safety/transition_safe_fixed_point.v` | `advance_epoch` | `crates/consensus/src/transition.rs` | PROVED |
| `phi_safety_monotone` | `proofs/safety/phi_safety_monotone.v` | Φ_safety evaluation in `advance_epoch` | `crates/consensus/src/lyapunov.rs` | PROVED |
| `phi_safety_aggregation_correct` | `proofs/safety/phi_safety_aggregation.v` | `evaluate_phi_safety()` | `crates/consensus/src/lyapunov.rs` | PROVED |
| `domain_isolation` | `proofs/safety/domain_isolation.v` | Domain A/B boundary | `crates/consensus/src/domain.rs` | PROVED |
| `state_root_binding` | `proofs/cascade/cascade_binding.v` | `compute_state_root()` / `ProjectedView::compute_root()` | `crates/consensus/src/transition.rs` | CI-VERIFIED |
| `causal_fingerprint` | `proofs/safety/causal_fingerprint.v` | fingerprint chain in `advance_epoch` | `crates/consensus/src/transition.rs:749` | PROVED |
| `cascade_avalanche_property` | `proofs/privacy/cascade_avalanche_property.v` | `h_cascade()` | `crates/consensus/src/cascade.rs` | STATISTICAL (not a v1.0 security proof) |

---

## 3. Empirical Evidence Layer

The CI pipeline provides 10+ test vectors that serve as executable witnesses for AX-2:

| Vector file | Contents | CI job |
|-------------|----------|--------|
| `tests/vectors/vectors.v1.json` | Fixed-point arithmetic, state-root commitment KAT, leaf-index vectors, epoch-transition sequences | `test-determinism` |
| `tests/vectors/cascade_kat.json` | QASH-CASCADE-7 known-answer test vectors for all 7 L1 primitives and the combined output | `cavp-kat` |

**Cross-ISA identity:** The `platform-determinism.yml` workflow (when active) confirms that the
cascade output and state root are byte-for-byte identical on x86_64, aarch64, and riscv64gc.

**Compiler differential:** The `multi-compiler-diff.yml` workflow confirms that opt-level=0 and
opt-level=3 (and the Cranelift backend) produce identical state roots, providing evidence against
optimizer-induced UB in Domain A code.

---

## 4. What AX-2 Claims and Does Not Claim

### Claims

- The Rust `advance_epoch` function produces the same state root as the Coq model on all
  authorized ISAs, as witnessed by CI vectors and multi-compiler differential testing.
- Arithmetic operations in Domain A code use the same semantics as the Coq model (checked
  arithmetic, no saturation, same rounding behavior for FixedPoint).
- The Domain A/B partition (no unsafe, no floats, no HashMap, no wall-clock in `crates/consensus/`)
  matches the partition assumed in the Coq proofs.

### Does Not Claim

- Formal extraction: The Rust code was hand-translated from the Coq model. No automated
  extraction tool was used. This is a known gap (see `proofs/COVERAGE.md` §AX-2 classification).
- Full semantic equivalence for all possible inputs: AX-2 is an axiom supported by empirical
  evidence, not a theorem. Post-genesis, a stronger form could be pursued via a verified
  extraction path (e.g., Coq → `coq-of-rust` or equivalent).
- PAL correctness: Domain B code is not covered by AX-2. The PAL is an implementation artifact;
  only Domain A code is proof-eligible.

---

## 5. Scope of Extraction Equivalence

The proof-to-code equivalence is scoped to **Domain A functions** in `crates/consensus/`:

| Function | Proof-eligible |
|----------|---------------|
| `advance_epoch` | Yes — primary transition function |
| `evaluate_phi_safety` | Yes |
| `evaluate_lyapunov` | Yes |
| `compute_state_root` / `ProjectedView::compute_root` | Yes |
| `h_cascade` | Yes |
| `h_domain` | Yes |
| `FixedPoint` arithmetic | Yes |
| PAL traits (`Time`, `Net`, `Attest`, `Halt`) | No — Domain B, runtime-specific |
| `CommitmentFrame` encode/decode | No — Domain B wire format |
| Threshold signing, ZK backend | No — scaffold/not-claimed |

---

## 6. Gap List (Post-Genesis Enhancement Targets)

| Gap | Description | Priority |
|-----|-------------|----------|
| AX-2 strengthening | Replace hand-translation with automated extraction check | Post-genesis |
| Blinding PRF game | Formal AU game proof for `derive_epoch_blinding_key` | Phase 3-A |
| IT-MAC forgery bound | Formal GF(2¹²⁸) AU game for cascade IT-MAC | Phase 3-B |
| ORAM access non-interference | Formal privacy proof for blinded access patterns | Phase 3-D |
| Avalanche (formal) | ROM formalization of cascade avalanche property | Non-blocking |
