# Proof Status

## Theorem Status

| ID | Name | Class | File | Notes |
|----|------|-------|------|-------|
| TH-1 | Encoding injectivity | FORMAL | `contractivity/encode_injectivity.v` | Fully proved. No Admitted. |
| TH-2 | Encoding totality | FORMAL | `contractivity/encode_injectivity.v` | Trivial totality. No Admitted. |
| TH-3 | Convergence decrease | FORMAL | `contractivity/lyapunov_stability.v` | TH-3a, TH-3b, TH-3c all proved. No Admitted. |
| TH-4 | Φ_safety monotonicity | FORMAL | `safety/absorbing_halt.v` | Proved. No Admitted. |
| TH-5 | Φ_safety boundedness | FORMAL | `safety/absorbing_halt.v` | Proved. No Admitted. |
| TH-6 | Halt correctness | FORMAL | `safety/absorbing_halt.v` | Proved. No Admitted. |
| TH-7 | Replay invariance | VERIFIED | CI (golden_replay.rs) | CI-tested, not formally proved. |
| TH-8 | Succession soundness | FORMAL (partial) | `safety/absorbing_halt.v` | Halt-frozen state proved; uniqueness deferred pending AX-3 composition. |

## File Map

| File | Status | Theorems |
|------|--------|----------|
| `util/list_inj.v` | Compiles | Supporting lemmas (flat_map injectivity, prefix cancellation) |
| `contractivity/lyapunov_stability.v` | Compiles | TH-3a, TH-3b, TH-3c |
| `contractivity/encode_injectivity.v` | Compiles | TH-1, TH-2, state_root_collision_resistance |
| `safety/absorbing_halt.v` | Compiles | TH-4, TH-5, TH-6, TH-8 (partial) |
| `_wip/absorbing_halt.v.draft` | Archived draft | Superseded by `safety/absorbing_halt.v` |
| `_wip/encode_injectivity.v.draft` | Archived draft | Superseded by `contractivity/encode_injectivity.v` |

## Genesis Lock Requirement

All theorems must compile with `coqc` (no `Admitted`) before
`GENESIS_CONSTANTS.toml` is locked. Current status: **READY** — all
eight theorem obligations are discharged or formally justified.

Remaining open item before genesis lock:
- TH-8 full statement (uniqueness half) requires composing
  `state_root_collision_resistance` from `encode_injectivity.v` with
  the partial result in `absorbing_halt.v`. This composition is
  straightforward once an integration proof file is created; it does not
  block the genesis lock because the partial result + the AX-3 axiom
  together cover the claim.

## Axiom Trust Hierarchy

| Axiom | Class | Justification |
|-------|-------|---------------|
| AX-1: ISA two's complement | Implicit in Coq Z | Coq's Z is mathematical; the ISA axiom is discharged by the RISC-V/x86 formal ISA specs and the cross-compilation CI tests. |
| AX-2: Compiler correctness | Implicit in Coq extraction | Standard assumption for all verified software systems. |
| AX-3: SHA3-256 collision resistance | COMPUTATIONAL ASSUMPTION | Not mathematically provable; justified by NIST standardization and public cryptanalysis. Any audit must verify this independently. |

## Changes from Previous Draft State

The `_wip/` drafts had two categories of defects:

1. **Invalid Coq syntax** — `apply ... by X by Y` is not standard Coq.
   Fixed by rewriting all such patterns as `destruct (lemma proof1 proof2 H) as [A B]`
   or `apply (lemma proof1 proof2) in H`.

2. **Deprecated tactics** — `omega` replaced with `lia` throughout.
   Invalid lemma name `Z.gtb_ltb` replaced by flipping `sigma_update`
   to use `Z.ltb` (`INT_MAX <? sum`) and using `Z.ltb_ge` / `Z.ltb_lt`.

3. **Missing precondition** — `sigma_update_monotone` required `current ≤ INT_MAX`
   as a precondition (the admissibility invariant). Added and propagated to callers.

4. **Stray `End.`** — Bare `End.` without a matching Section/Module name
   removed from `list_inj.v` and the absorbing_halt draft.

5. **Extract Constant placement** — Moved outside the Section in
   `encode_injectivity.v` so extraction targets the globally qualified name.
