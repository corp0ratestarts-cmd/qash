# QASH Profile Taxonomy

**Status:** Normative  
**Scope:** Protocol profile definitions and boundary rules.  
**Related:** `ADR-015-pure-qash-repository-split.md`, `docs/spec/09_privacy_model.md`

---

## §19.0 — Purpose

QASH is not a single monolithic deployment target. The protocol defines a layered set of
**profiles** that share the same Domain A deterministic consensus core but differ in:

- Which observer classes are present (see `09_privacy_model.md`)
- Whether a genesis-authorised disclosure key exists
- Whether evidence retention is permitted for compliance purposes
- Whether attested hardware boundaries are required
- Which genesis constants apply

This document defines three normative profiles. Each profile has a hard list of what
it MUST and MUST NOT contain. Mixing profile concepts across repos or builds is a
protocol violation.

---

## §19.1 — Profile Registry

### Profile 1 — Pure QASH Core

**Repository:** `corp0ratestarts-cmd/pure-qash`  
**Identity:** Privacy-maximal graph-non-publishing digital cash protocol.

```
Pure QASH Core =
  graph-non-publishing protocol
  + zero-persistence production profile
  + QASH Constitutional Scarcity Axiom
  + MEV-null Domain A economic surface
  + no regulatory disclosure key
  + no user graph evidence retention
  + no monetary governance
```

**MUST contain:**

- Class I (public observer), Class II (authorized validator), Class III (receipt holder)
  observer classes only
- Deterministic constitutional scarcity (decaying issuance, fixed tail, 100% fee burn,
  100% slash burn)
- Zero-persistence production mode (no raw graph material in WAL, logs, metrics, or
  public channels)
- MEV-null Domain A economic surface (deterministic resource-cost fees, conflict
  annihilation)
- Note/nullifier transfer semantics (nullifier sets, no stable user identifiers)
- Blind certification evidence only (proves control behavior, not user behavior)
- Own genesis constants, separate from umbrella QASH

**MUST NOT contain:**

- Class IV (regulatory authority) observer class
- Genesis-authorised disclosure key
- Lawful-basis disclosure flows (`lawful_basis`, `disclosure_domain`)
- Regulated receipt disclosure
- Priority fees, base-fee/tip splits, fee auctions, mempool ordering
- Validator fee revenue
- Monetary governance (oracle-based supply adjustment, discretionary treasury)
- Raw transaction retention in any production path
- Peer IP or routing metadata in any durable store
- Serialization of ephemeral admission envelopes

**Privacy claim:** See `docs/spec/09_privacy_model.md` in the pure-qash repo.  
**Proof requirement:** TH-P1 and TH-P2 are required evidence gates before genesis-candidate.

---

### Profile 2 — QASH Regulated Profile

**Repository:** `corp0ratestarts-cmd/qash` (umbrella)  
**Identity:** Optional regulated deployment tier with scoped lawful disclosure.

```
QASH Regulated Profile =
  Pure QASH Core privacy model
  — (with reduction in privacy claim)
  + genesis-authorised disclosure key
  + Class IV (regulatory authority) observer
  + epoch-scoped regulated receipt access
  + lawful-basis disclosure flows
  + jurisdiction-specific compliance evidence
```

**Observer classes present:** Class I, II, III, IV  
**Disclosure key:** Present, genesis-authorised, epoch-scoped  
**Privacy claim:** Reduced relative to Pure QASH Core; forward secrecy preserved within
epoch-scoped disclosure bounds.

**Permitted additions over Pure QASH Core:**

- Class IV observer scaffolding
- Regulated receipt decryption with lawful-basis gate
- Compliance documentation referencing user-activity evidence structures
- Disclosure domain configuration in genesis constants

**Constraint:** The regulated profile MUST NOT appear in the Pure QASH repo.
Any PR to `corp0ratestarts-cmd/pure-qash` that introduces regulated-profile concepts
MUST be blocked by absence guards and rejected at review.

---

### Profile 3 — QASH Sovereign Hardened Profile

**Repository:** `corp0ratestarts-cmd/qash` (umbrella, post-v1 research track)  
**Identity:** Pure QASH privacy model with attested hardware admission boundary.

```
QASH Sovereign Hardened Profile =
  Pure QASH Core
  + attested DPU/SmartNIC admission boundary
  + Confidential Computing host (TDX/SEV-SNP/ARM CCA)
  + IOMMU lockdown and hardware-backed storage erasure
  + HSM/TPM key anchoring for validator identity
  + zero-persistence hardware assurance evidence
```

**Observer classes present:** Class I, II, III (same as Pure QASH Core)  
**Disclosure key:** None (maintains Pure QASH privacy model)  
**Privacy claim:** Same as Pure QASH Core, with additional hardware assurance evidence.

**Distinction from Pure QASH Core:** The privacy model is identical. The difference is
in Domain B: Sovereign Hardened adds attested hardware boundaries for the admission path.
This is a deployment-tier distinction, not a protocol distinction.

**Current status:** Post-v1 research track. Hardware attestation stubs exist in the
umbrella repo. Not required for Pure QASH Core genesis.

---

## §19.2 — Profile Boundary Rules

### Rule PB-1 — No cross-profile contamination

A build artifact MUST belong to exactly one profile. A binary compiled with Regulated
Profile features MUST NOT claim Pure QASH Core privacy properties, and vice versa.

### Rule PB-2 — Absence guards enforce Pure QASH boundary

The pure-qash repo CI runs absence guards that fail if any Regulated or Sovereign
Profile concept leaks into the codebase. See `scripts/check_pure_absence_guards.sh`
in the pure-qash repo.

### Rule PB-3 — Import review required

Any import of code from the umbrella repo into pure-qash requires a PR review that:
1. Verifies the imported code passes Pure QASH absence guards
2. Records the import in `docs/release/import_manifest.md` with source commit SHA

### Rule PB-4 — No silent upstream tracking

Pure QASH does not automatically track umbrella QASH. Any sync from the umbrella
to pure-qash is an explicit import PR subject to Rule PB-3.

### Rule PB-5 — Genesis constants are profile-specific

Pure QASH Core genesis constants live only in `corp0ratestarts-cmd/pure-qash/GENESIS_CONSTANTS.toml`.
The umbrella repo's `GENESIS_CONSTANTS.toml` is not modified for Pure QASH content.

---

## §19.3 — Profile Comparison Table

| Property | Pure QASH Core | QASH Regulated | QASH Sovereign Hardened |
|---|---|---|---|
| Repository | pure-qash | qash (umbrella) | qash (umbrella, post-v1) |
| Observer classes | I, II, III | I, II, III, IV | I, II, III |
| Disclosure key | None | Genesis-authorised | None |
| Lawful-basis flows | Absent | Present | Absent |
| Priority fees | Absent | Absent | Absent |
| Fee burn | 100% (total) | 100% (total) | 100% (total) |
| Zero-persistence mode | Required | Required | Required + HW evidence |
| Raw TX retention (production) | Forbidden | Forbidden | Forbidden |
| MEV-null Domain A | Required | Required | Required |
| Attested HW admission | Optional | Optional | Required |
| Compliance evidence | Blind only | Structured (lawful basis) | Blind + HW attestation |
| Genesis constants | Own file | Own file | Own file |
| Privacy claim | Maximum | Reduced (Class IV) | Maximum |

---

## §19.4 — Profile Admission for Future Profiles

A new profile may be defined only by:

1. A new ADR recording the profile name, repo target, observer classes, and boundary rules
2. A traceability row linking the ADR to this taxonomy document
3. An explicit decision on whether the new profile is a Pure QASH derivative (must
   pass Pure QASH absence guards) or an umbrella derivative (may contain regulated/disclosure
   concepts but must not claim Pure QASH privacy properties)

No profile may claim Pure QASH Core privacy properties unless it passes the Pure QASH
absence guards and has no Class IV observer class or genesis-authorised disclosure key.
