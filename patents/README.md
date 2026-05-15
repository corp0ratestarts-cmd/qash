# Patent Evidence Support

This directory is a technical evidence pack for counsel and inventors. It is not
legal advice, a patentability opinion, or a substitute for prior-art searching.
It organizes QASH mechanisms into reproducible, implementation-specific records
that can be reviewed before any patent filing decision.

## Candidate invention families

| Family | Disclosure | Primary implementation anchors |
| --- | --- | --- |
| Deterministic replay isolation | `deterministic-replay/INVENTION_DISCLOSURE.md` | `docs/spec/00_execution_model.md`, `crates/consensus/`, `crates/pal/` |
| Lyapunov consensus stability | `lyapunov-consensus/INVENTION_DISCLOSURE.md` | `docs/spec/01_consensus.md`, `crates/consensus/src/lyapunov.rs`, `crates/consensus/src/transition.rs` |
| Cross-ISA equivalence | `cross-isa-equivalence/INVENTION_DISCLOSURE.md` | `docs/spec/00_execution_model.md`, `scripts/verify_two_stage_build.sh`, CI determinism checks |

## Evidence map

- Diagrams: `diagrams/`
- Prior-art differentiation templates: `prior_art/`
- Claim-support traceability: `claim_support/`
- Replay evidence capture location: `../artifacts/replay_equivalence/`
- Benchmark evidence capture location: `../artifacts/benchmarks/`
- Threat model: `../docs/threat_model/nondeterminism.md`
- Architecture decision records: `../docs/adr/`

## Filing caution

Public commits, README updates, talks, issues, and papers can be public
disclosures. Some jurisdictions have no grace period after disclosure. Review
this evidence pack with qualified software patent counsel before relying on it
for filing strategy.
