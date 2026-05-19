# Proof Obligation Pipeline

## Operational flow

1. Developer edits consensus code.
2. CI invokes `scripts/compile_proof_obligations.py`.
3. Script computes changed files from git diff (PR base SHA when available).
4. Script maps sensitive Domain A regions (`transition.rs`, `lyapunov.rs`, `causal_order.rs`) to theorem/proof tags.
5. If any mapped region changed, `proofs/COVERAGE.md` must be updated in the same delta.
6. Script enforces policy: no merge when Domain A changes introduce untracked obligations.
7. Script emits machine-readable `proofs/coverage.json` generated from `proofs/COVERAGE.md`.

## Policy rule

PRs touching Domain A semantics cannot merge if proof obligations implied by the changed region are not tracked in `proofs/COVERAGE.md`.

## Inputs

- `proofs/obligation_map.toml`
- `proofs/COVERAGE.md`
- git diff (`GITHUB_BASE_SHA...HEAD` for PRs)

## Outputs

- obligation summary printed as JSON
- `proofs/coverage.json`
