# Semantic Kernel Compression Plan

## Problem

The repository is accumulating governance complexity faster than semantic compression.

Recent PRs added:
- replay orchestration governance,
- semantic-scope governance,
- stabilization governance,
- dependency governance,
- replay wrapper restrictions,
- CI tier expansion.

These PRs are attempting to protect the same invariant:

> identical replay produces identical state.

The architectural issue is that replay semantics are currently distributed across:
- scripts,
- CI policy,
- orchestration wrappers,
- contributor governance,
- dependency policy,
- runtime integration boundaries.

This increases review complexity and weakens auditability.

---

## Architectural Compression Goal

Replace governance-heavy replay coordination with a mechanically enforced semantic kernel.

Target invariant:

```text
ONE semantic kernel
MANY operational shells
```

The kernel becomes the only authoritative source for:
- deterministic replay,
- canonical serialization,
- transition semantics,
- deterministic ordering,
- state root derivation,
- epoch transitions,
- admissibility evaluation.

---

## Proposed Structure

```text
crates/kernel/
```

The kernel crate should absorb consensus-critical deterministic semantics currently spread across:
- replay orchestration,
- transition logic,
- serialization pathways,
- canonical hashing boundaries,
- deterministic ordering layers.

The kernel must remain:
- no_std,
- deterministic,
- bounded,
- replay-pure,
- environment-independent.

---

## Mechanical Governance Replacement

### Rule 1 — Kernel Boundary

Any modification under:

```text
crates/kernel/
```

is consensus-critical.

Mandatory CI:
- cross-ISA replay,
- proof verification,
- vector parity,
- deterministic replay corpus.

---

### Rule 2 — Non-Kernel Isolation

Everything outside the kernel is non-semantic.

This includes:
- PAL,
- networking,
- orchestration,
- benchmarking,
- proof transport,
- persistence,
- tooling.

Non-kernel code must never influence canonical state roots.

---

### Rule 3 — Canonical Replay Interface

Replay semantics must not exist in shell wrappers.

Canonical interface:

```rust
kernel::replay(trace) -> canonical_root
```

All replay tooling should call the same deterministic implementation.

This removes the need for replay-wrapper governance and replay-entrypoint proliferation controls.

---

## Immediate Simplifications

Once the kernel boundary exists:

The repository can remove or drastically reduce:
- replay orchestration governance,
- semantic-scope governance complexity,
- replay-wrapper policy,
- duplicate replay entrypoints,
- large portions of stabilization policy.

The system becomes structurally deterministic instead of procedurally deterministic.

---

## Migration Sequence

### Phase 1
- Introduce `crates/kernel/`.
- Move replay semantics into the kernel.
- Define canonical replay API.

### Phase 2
- Make CI derive semantic criticality from path ownership.
- Replace governance-heavy PR semantic rules with path-based enforcement.

### Phase 3
- Collapse replay wrapper scripts into thin invocations of kernel replay.
- Remove duplicate replay semantics.

### Phase 4
- Reduce governance surface area.
- Preserve only:
  - kernel invariants,
  - domain isolation,
  - proof obligations,
  - reproducibility guarantees.

---

## Expected Benefits

- Smaller trusted semantic surface.
- Reduced audit complexity.
- Easier proof refinement.
- Reduced governance entropy.
- Clearer consensus-critical ownership.
- Stronger replay guarantees.
- Lower contributor cognitive load.
- Improved certification posture.
- Simpler CI topology.
- Elimination of replay semantic leakage into tooling.

---

## Alignment With QASH Goals

QASH is strongest when deterministic semantics are mechanically inevitable.

The repository should therefore optimize for:

```text
semantic compression
```

rather than:

```text
governance expansion
```
