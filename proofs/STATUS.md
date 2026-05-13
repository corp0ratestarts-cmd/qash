# Proof Status

`proofs/STATUS.md` is the canonical source of theorem/proof status for this repository.

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

## Proof done gate (required for status = CLOSED)

A theorem may be marked CLOSED only when all conditions are met:

1. The theorem statement and supporting lemmas compile via `coqc`.
2. The theorem's proof tree has no `Admitted`/`Axiom` placeholders in its dependency chain.
3. CI runs `coqc` for the touched proof targets and passes on the pinned toolchain.
4. Any status table outside this file is updated in the same PR.
