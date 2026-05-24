# Stabilization Governance

This document defines governance rules for stabilizing QASH from evolving implementation to reproducible, auditable protocol behavior.

## Canonical ownership map

Ownership is by subsystem authority (who decides semantic changes) and by review responsibility (who must review protocol-affecting edits).

| Surface | Canonical path(s) | Primary owner(s) | Required reviewers for semantic changes |
|---|---|---|---|
| Model bridge | `model/` | Model + formal methods maintainers | Consensus + proofs |
| Deterministic consensus | `crates/consensus/` | Consensus maintainers | Consensus + PAL (boundary validation) |
| PAL / hosted runtime | `crates/pal/`, `src/` (hosted integration) | PAL maintainers | PAL + consensus (ingress canonicalization) |
| Vector runner and corpus harness | `tests/vector-runner/`, `tests/vectors/`, `crates/*/tests/` | Verification maintainers | Subsystem owner(s) impacted by vectors |
| Replay scripts and replay CI helpers | `scripts/` replay utilities, replay workflows in `.github/workflows/` | Release/verification maintainers | Consensus + PAL |
| Proof artifacts and proof status | `proofs/` | Proof maintainers | Proofs + owning subsystem maintainer |
| Genesis constants and lock artifacts | `GENESIS_CONSTANTS.toml`, `spec/genesis-artifacts.txt` and associated verification scripts | Release leads | Consensus + proofs + release lead |

When ownership is unclear, default to the stricter rule: require sign-off from both the producing side and the consuming side of the boundary.

## CI tiers

### Tier 1 — merge-blocking semantic gates

Tier 1 must pass before merge. These checks protect protocol semantics and replay determinism.

- Deterministic consensus tests and affected replay/vector tests.
- Cross-ISA determinism checks for supported targets (where configured).
- Genesis-hash and lock-integrity guards when relevant files are touched.
- Proof compilation gates required by current policy for touched proof-sensitive areas.
- Any check that can change accepted/rejected protocol behavior, state-root outputs, or canonical encodings.

### Tier 2 — advisory security checks

Tier 2 runs by default and is expected to be triaged, but does not automatically block merge unless promoted.

- Supply-chain and dependency risk checks (for example, deny/audit policy outputs).
- Extended fuzz/smoke security probes.
- Static or policy checks that identify risk signals without proving semantic breakage.

Findings must be dispositioned in the PR (fix, defer with rationale, or escalate to a blocking follow-up).

### Tier 3 — non-blocking hygiene

Tier 3 improves repository quality and contributor ergonomics; failures do not imply protocol breakage.

- Formatting, spelling, docs lint, and similar hygiene checks.
- Best-effort quality checks for non-critical documentation and metadata.

Tier 3 failures should still be addressed promptly, but they are not semantic gates.

## PR semantic-scope rule

Prefer one semantic axis per PR. Examples of semantic axes include:

- consensus transition semantics,
- canonical encoding/wire format,
- replay corpus expectations,
- proof obligations,
- PAL boundary/canonicalization behavior,
- genesis constants.

If a PR must span multiple axes, keep it explicitly coupled and justified (for example, an encoding change plus the exact vector/proof updates that make it verifiable). Avoid mixing unrelated semantic changes with refactors or hygiene edits.

## Dependency governance

Consensus- and serialization-critical crates must use exact pins.

- Use exact versions (for example, `=x.y.z`) for crates that influence:
  - deterministic arithmetic,
  - hashing/digest behavior,
  - canonical serialization/deserialization,
  - state-root construction,
  - replay transcript encoding.
- Do not rely on implicit minor/patch drift for critical crates.
- Any bump to a critical crate requires:
  1. rationale in the PR,
  2. replay/vector evidence that outputs are unchanged (or intentionally changed and re-pinned),
  3. explicit review by the owning subsystem maintainer(s).
- Non-critical tooling dependencies may follow normal workspace policy but should still prefer conservative upgrades near genesis lock windows.

## Boundary rules for replay-critical paths

The following are forbidden in replay-critical paths (including consensus transitions, canonical encoding, vector evaluation, and replay acceptance checks):

- Reading wall-clock or monotonic time for semantic decisions.
- Reading host entropy/randomness for semantic decisions.
- Branching on environment/process attributes (locale, timezone, CPU features, hostnames, env vars, filesystem ordering, network timing) when such branching can affect protocol outputs.
- Any nondeterministic map/set iteration that can influence committed outputs.

All protocol-affecting inputs must be canonicalized at ingress and recorded in replayable form. Domain B observations are allowed only when converted into deterministic, canonical inputs before crossing into Domain A semantics.
