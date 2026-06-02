# QASH Proof-to-Code Refinement

This document explains the correspondence between the Coq formal model
(`proofs/model/Model.v`) and the Rust implementation
(`crates/consensus/src/transition.rs`), and how to independently verify it.

---

## Three-layer correspondence chain

```
Layer 1: Coq model properties (formally proved)
         proofs/model/RefinementStatement.v
         RT-1 … RT-4 + corollaries via AX2_rust_refinement

         │
         ▼

Layer 2: Test-vector alignment (proved by Coq reflexivity)
         proofs/model/Model.v §6
         Examples: advance_epoch_idle4_observation_checked,
                   advance_epoch_lyapunov_halt_observation_checked, etc.

         │
         ▼

Layer 3: Rust conformance (CI-verified)
         crates/consensus/tests/coq_vectors.rs
         12 test vectors (TV-0..TV-11) in proofs/model/vectors.json
         asserted against advance_epoch() on every CI run
```

The gap between Layer 2 and Layer 3 is covered by **AX2_rust_refinement**
(see §Axiom stack below).

---

## What is proved (Layer 1)

| Theorem | File | Statement |
|---------|------|-----------|
| `RT1_successful_step` | `RefinementStatement.v` | If the state is not halted, the update is valid, and δ_window ≤ ε, then epoch advances by 1 and halt flag is cleared |
| `RT2_halt_step` | `RefinementStatement.v` | If the state is not halted, the update is valid, and δ_window > ε, then the halt flag is set |
| `RT3_halt_absorbing_epoch` | `RefinementStatement.v` | If the state is already halted, the epoch is unchanged |
| `RT4_halt_absorbing_flag` | `RefinementStatement.v` | If the state is already halted, the halt flag remains set |
| `rust_RT1` … `rust_RT4` | `RefinementStatement.v` | The same four properties lifted to the Rust implementation via AX2_rust_refinement |

These theorems are proved from the Coq model alone — no Rust semantics are needed.

---

## Coq-to-Rust definition mapping

| Coq identifier | Rust identifier | File |
|----------------|-----------------|------|
| `step` / `advance_epoch` | `advance_epoch` | `src/transition.rs` |
| `run` | loop over `advance_epoch` | `src/transition.rs` |
| `ValidatorMetrics` | `ValidatorMetrics` | `src/lyapunov.rs` |
| `vm_D`, `vm_C`, `vm_S` | `.divergence`, `.conflict`, `.slash_accum` | `src/lyapunov.rs` |
| `v_validator` | `ValidatorMetrics::lyapunov_value` | `src/lyapunov.rs` |
| `v_sum` | sum loop in `evaluate` | `src/lyapunov.rs` |
| `ConvWindow` / `cw_push` | `ConvergenceWindow` / `push` | `src/lyapunov.rs` |
| `delta_window` | `ConvergenceWindow::delta_window` | `src/lyapunov.rs` |
| `is_halted` | `HaltReason::is_halt` | `src/transition.rs` |
| `apply_updates` | `step_1_validate` + `step_2_apply` | `src/transition.rs` |
| `fixed_mul` | `FixedPoint::checked_mul` | `src/fixed_point.rs` |
| `weight_D` / `epsilon` | `WEIGHT_D` / `EPSILON` | `src/lyapunov.rs` |
| `scale` | `SCALE` | `src/fixed_point.rs` |

All constants match `GENESIS_CONSTANTS.toml`:
- `scale = 1_000_000`
- `weight_D = 400_000`, `weight_C = 350_000`, `weight_S = 250_000`
- `epsilon = 20_000`, `window_sz = 3`

---

## Extraction pipeline

Coq's extraction facility produces an OCaml module that is semantics-preserving
for the pure functional fragment used in `Model.v`.

```bash
# From the repo root:
cd proofs

# 1. Compile Model.v (prerequisite)
coqc -Q . QASH model/Model.v

# 2. Run extraction — produces model_extracted.ml in proofs/
coqc -Q . QASH model/Extract.v

# 3. Compile the extracted OCaml (requires zarith)
#    Install: opam install zarith
ocamlfind ocamlopt \
  -package zarith -linkpkg \
  model_extracted.ml -o model_extracted
```

The extracted `step` and `run` functions are byte-for-byte equivalent to
the Coq definitions (no `Admitted` markers, no floating point, no side effects).
An independent auditor can apply these functions to the test vectors in
`proofs/model/vectors.json` and compare outputs against `proofs/model/transition_observations.json`.

---

## Axiom stack

| Axiom | Where | Justification |
|-------|-------|---------------|
| `AX2_rust_refinement` | `RefinementStatement.v` | Empirical: 12 CI test vectors (TV-0..TV-11) verify the Rust output matches the Coq model. Trust reduces to AX-2 (rustc 1.95.0 correctness). To strengthen: add more vectors or use a Rust-in-Coq embedding. |
| `AX-2` (external) | CLAUDE.md / COVERAGE.md | Rust compiler correctness — standard for any compiled-language formal project |
| `AX-3` (external) | `cascade_collision_resistance.v` | SHA3-256 collision resistance — standard cryptographic assumption |
| ZArith soundness | Coq standard library | Standard assumption for any Coq proof using integers |

`AX2_rust_refinement` appears in `proofs/COVERAGE.md` under the
"Refinement" section and is tracked by `scripts/check_axiom_coverage.sh`.

---

## Strengthening the refinement

The current axiom is supported by 12 test vectors (TV-0..TV-11). To reduce trust further:

1. **More test vectors**: Add cases to `proofs/model/vectors.json` and
   corresponding runners to `coq_vectors.rs`. Each additional vector tightens
   the empirical support.

2. **Property-based testing**: Extend `proptest` suites in
   `crates/consensus/tests/golden_replay.rs` to cover random inputs and assert
   the Rust output matches the Coq-extracted OCaml's output.

3. **Rust-in-Coq embedding**: Use RustBelt or a K-Rust semantics to
   formally prove the Rust source implements the Coq model. This is a
   substantial research effort deferred to post-v1.1.

---

## Running the CI proof check

```bash
# Verify Model.v and RefinementStatement.v both compile with zero Admitted markers:
cd proofs
coqc -Q . QASH model/Model.v
coqc -Q . QASH model/RefinementStatement.v

# Verify coq_model_parity test vectors:
cargo test -p qash-consensus coq_model_parity -- --nocapture
```
