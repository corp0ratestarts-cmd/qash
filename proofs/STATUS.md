# Proof Status

The Coq files in `_wip/` are design sketches, not verified proofs.
They were drafted to capture proof obligations and theorem structure
but have not been mechanically verified by `coqc`.

## Theorem Status

| ID | Name | Class | Compiles | Notes |
|----|------|-------|----------|-------|
| TH-1 | Encoding injectivity | FORMAL | ❌ | Sketch in `_wip/encode_injectivity.v.draft` |
| TH-2 | Encoding totality | FORMAL | ❌ | Trivial; will compile once TH-1 framework is fixed |
| TH-3 | Convergence decrease | FORMAL | — | Not started |
| TH-4 | Φ_safety monotonicity | FORMAL | ❌ | Sketch in `_wip/absorbing_halt.v.draft` |
| TH-5 | Φ_safety boundedness | FORMAL | ❌ | Sketch in `_wip/absorbing_halt.v.draft` |
| TH-6 | Halt correctness | FORMAL | ❌ | Sketch in `_wip/absorbing_halt.v.draft` |
| TH-7 | Replay invariance | VERIFIED | — | CI-tested, not formally proved |
| TH-8 | Succession soundness | FORMAL | ❌ | Depends on TH-1 |

## Why files were moved to _wip/

The drafts use `apply ... by X by Y` syntax which isn't valid Coq,
and have multiple scope-mixing issues between `Z` and `nat`.
They capture the correct proof *strategy* but cannot be compiled
without significant rewriting that should be done by someone with
direct Coq experience, ideally interactively with `coqide` or `Proof General`.

## Genesis lock requirement

These proofs must be discharged (no `Admitted`, compiles with `coqc`)
before `GENESIS_CONSTANTS.toml` is locked. Until then, theorems are
specification-level claims, not formal guarantees.


## Adversarial theorem obligations (tracked)

- TH-A1 Partition Safety: **planned**
  - Non-goals: no progress guarantee during active partition.
- TH-A2 Reordering Safety: **planned**
  - Non-goals: no FIFO / exactly-once transport guarantees.
- TH-A3 Byzantine Boundary Safety: **planned**
  - Non-goals: does not prevent adversarial attempts; proves deterministic rejection/accounting.
- TH-A4 Conditional Liveness: **planned**
  - Assumptions required: eventual delivery, healed partition, sufficient active honest validators.

CI simulation rollout:
- Phase 1: non-blocking adversarial scenario hook enabled in CI.
- Phase 2: promote to blocking after stability baseline is met.
