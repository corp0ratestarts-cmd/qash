# Claim Support Map: Deterministic Replay Isolation

This map links candidate claim elements to repository evidence. It is intended
to help inventors and counsel find technical support quickly; it is not a claim
chart or legal opinion.

| Candidate claim element | Source support | Diagram / artifact support | Proof or test support |
| --- | --- | --- | --- |
| Deterministic and nondeterministic execution domains | `docs/spec/00_execution_model.md`; `crates/consensus/`; `crates/pal/` | `patents/diagrams/replay_isolation_sequence.mmd` | Future boundary-flow tests |
| Boundary rule preventing Domain B values from altering Domain A transitions | `docs/spec/00_execution_model.md`; `README.md` Domain A / Domain B rules | `patents/diagrams/replay_isolation_sequence.mmd` | Static lint and negative replay vectors to add |
| Admissibility gate before transition | `crates/consensus/src/transition.rs`; `docs/spec/01_consensus.md` | `patents/diagrams/state_transition_machine.mmd` | `crates/consensus/tests/golden_replay.rs` |
| Checked arithmetic with absorbing halt | `docs/spec/00_execution_model.md`; `crates/consensus/src/fixed_point.rs`; `crates/consensus/src/transition.rs` | `patents/diagrams/state_transition_machine.mmd` | Fixed-point and transition tests |
| Canonical encoding and domain-tagged state roots | `crates/consensus/src/encoding.rs`; `crates/consensus/src/hash.rs` | Replay artifact root checkpoints | Golden replay root comparisons |
| Cross-ISA replay conformance | `scripts/verify_two_stage_build.sh`; CI platform determinism workflow | `artifacts/replay_equivalence/README.md` | Cross-target CI logs to archive |

## Evidence gaps to close

1. Add negative tests for attempted Domain B contamination paths.
2. Archive CI state-root outputs for at least two architecture targets.
3. Add generated replay bundles containing input bytes, encoded states, roots,
   target triples, compiler versions, and spec hash.
4. Add a runtime/model equivalence trace once the executable formal model is
   available.
