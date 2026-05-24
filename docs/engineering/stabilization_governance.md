# Stabilization Governance

## Workspace dependency pinning policy

The root workspace manifest owns exact version pins for shared runtime-semantic crates via `[workspace.dependencies]`. Current baseline:

- `serde = =1.0.228`
- `serde_json = =1.0.145`
- `sha3 = =0.10.8`

These are exact (`=`) pins because serialization and hashing behavior are protocol-adjacent and must not drift implicitly across crates.

## Consensus-critical and serialization-critical boundaries

- **Consensus-critical path (Domain A):** deterministic crates (primarily `crates/consensus`) must keep `default-features = false` on shared hashing/serialization dependencies unless there is an explicit deterministic justification.
- **Runtime/hosted path (Domain B):** hosted or tooling crates may opt into standard-library features where required, but should still source shared crate versions from workspace pins.
- **Manifest consumption rule:** workspace members should use `workspace = true` for governed shared crates, and only override feature flags locally.

## Tooling vs runtime-semantic dependencies

- Dependencies that can influence protocol semantics (hashing, encoding, canonical serialization) are runtime-semantic and must be pinned and reviewed as protocol-impacting changes.
- Tooling/dev-only dependencies (bench, fuzz, test harness, lint, CI utilities) must remain segregated in `dev-dependencies`, standalone tool manifests, or feature-gated non-consensus paths.
- Dev/tooling crates must not leak feature activation into Domain A runtime artifacts.

## Change control for pin updates

Any update to pinned shared crates requires:

1. Explicit PR note stating which pins changed and why.
2. Impact assessment for determinism, serialization/canonicalization, and replay compatibility.
3. Local verification covering at minimum:
   - `cargo test -p qash-consensus --no-default-features`
   - affected PAL/tests when Domain B manifests changed.
4. If behavior-affecting, accompanying docs/spec traceability update in `docs/`.

Pin updates should be batched minimally and kept as one logical change per PR to simplify audit and rollback.
