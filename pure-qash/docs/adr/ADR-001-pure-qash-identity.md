# ADR-001 — Pure QASH Identity and Repository Governance

**Status:** Accepted  
**Date:** 2026-06-03  
**Scope:** Repository identity, profile boundary, import policy

---

## Context

Pure QASH is the privacy-maximal profile of the QASH protocol family. It is developed
in this repository (`corp0ratestarts-cmd/pure-qash`) as a separate, self-contained artifact
distinct from the umbrella repository (`corp0ratestarts-cmd/qash`).

The split is governed by ADR-015 in the umbrella repo and the profile taxonomy at
`docs/spec/19_profile_taxonomy.md` in the umbrella repo.

---

## Decision

This repository is **Pure QASH Core** as defined in the profile taxonomy.

**Identity:**
```
Pure QASH is graph-non-publishing digital cash with constitutional scarcity,
zero user-evidence persistence, and no endogenous Domain-A MEV surface.
```

**Invariants that can never be relaxed in this repo:**

1. No Class IV (regulatory authority) observer class
2. No genesis-authorised disclosure key
3. No lawful-basis disclosure flows
4. No priority fees or fee auctions
5. No validator fee revenue
6. No monetary governance
7. No raw user graph material in any production durable store
8. `EphemeralEnvelope` is non-serializable, non-cloneable, non-debuggable
9. All certification evidence is blind (proves control behavior, not user behavior)

**These are enforced by CI absence guards, not by convention.**

---

## No Upstream Auto-Tracking

Pure QASH does NOT automatically receive changes from the umbrella repo.
Any import must be reviewed, absence-guard-verified, and recorded in
`docs/release/import_manifest.md`.

Rationale: convenience imports are the primary risk vector for regulated-profile
concepts leaking into Pure QASH.

---

## Acceptance

- [x] This repo has no Class IV anywhere
- [x] This repo has no disclosure key anywhere
- [x] CI absence guards run on every PR
- [x] Import manifest exists and is updated on every import
