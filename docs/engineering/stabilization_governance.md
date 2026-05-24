# Stabilization Governance

QASH prioritizes deterministic semantic correctness over infrastructure convenience.

## Canonical ownership

- `model/`: semantic transition model
- `crates/consensus/`: replay semantics
- `crates/pal/`: persistence and ingress
- `crates/vector-runner/`: replay diagnostics
- `scripts/replay_test.sh`: replay orchestration
- `proofs/`: formal artifacts
- `GENESIS_CONSTANTS.toml`: genesis invariants

Duplicate orchestration paths should be removed rather than maintained in parallel.

## CI tiers

Tier 1 (merge blocking):
- replay determinism
- corpus pinning
- `no_std` builds
- model conformance
- consensus invariants
- cross-ISA replay checks

Tier 2 (advisory):
- OSV
- SBOM
- Scorecard
- dependency drift

Tier 3 (non-blocking hygiene):
- formatting
- docs
- clippy style lints
- geiger
- semver advisories

## Pull request scope

Pull requests should change one semantic axis at a time whenever possible.

Avoid combining these axes inside a single PR:
- replay semantics
- CI restructuring
- dependency policy
- crypto abstraction
- infrastructure refactors
- `no_std` migration

If a multi-axis change is unavoidable, include explicit rationale and reviewer acknowledgment in the PR body.

## Dependency governance

Consensus-critical and serialization-critical crates should prefer exact version pinning.

Tooling dependencies should remain isolated from deterministic runtime paths.

Workspace-level dependency authority should be preferred for shared critical crates to reduce drift.

## Boundary rules

The semantic core must not depend on:
- CI logic
- environment-specific behavior
- host-specific filesystem assumptions
- nondeterministic timing behavior

Replay-critical execution paths must remain deterministic across:
- `x86_64`
- `aarch64`
- `riscv64gc`

## Replay orchestration policy

`scripts/replay_test.sh` is the canonical replay orchestration entrypoint.

New replay wrappers or alternate CI replay entrypoints should only be introduced when replacing an existing path. Parallel orchestration paths should not be maintained.
