# ADR-015 — Pure QASH Repository Split

**Status:** Accepted  
**Date:** 2026-06-03  
**Authors:** Protocol design  
**Scope:** Repository and profile governance

---

## Context

The `corp0ratestarts-cmd/qash` repository has grown to encompass multiple distinct
deployment profiles: the privacy-maximal Pure QASH core, a Regulated profile that
permits scoped lawful disclosure, and a Sovereign Hardened profile that adds attested
hardware boundaries. These profiles have different privacy claims, different observer
classes, different genesis constants, and different threat models.

Placing all profiles in a single repository creates several problems:

1. **Claim contamination.** Pure QASH's strongest claim is that it has no Class IV
   (regulatory authority) observer, no disclosure key, and no user-evidence retention.
   If the umbrella repo also contains Class IV scaffolding, absence guards cannot be
   clean, and external auditors cannot verify the Pure QASH claim without auditing the
   entire umbrella.

2. **Genesis constant separation.** Pure QASH requires its own genesis constants,
   proof corpus, CI gates, and release path. Shared genesis constants would conflate
   the two networks.

3. **CI gate pollution.** Absence guards (rejecting `lawful_basis`, `disclosure_key`,
   `priority_fee`, etc.) cannot run in a repo that legitimately contains those concepts
   for the Regulated profile.

4. **Audit surface.** A pure-privacy-maximal system should be auditable as a
   self-contained artifact. The umbrella repo's compliance, sovereign, and regulated
   documentation adds audit burden that is not relevant to Pure QASH.

---

## Decision

Pure QASH will be developed as a separate minimal repository:

```
corp0ratestarts-cmd/pure-qash
```

The current `corp0ratestarts-cmd/qash` repository remains the **umbrella repository**
for all broader profile work:

- Regulated profile (Class IV disclosure, lawful-basis flows)
- Sovereign Hardened profile (attested DPU/SmartNIC/TEE/HSM boundary)
- Compliance artifacts and evidence matrices
- Post-v1 extension research
- Regulated receipt disclosure experiments
- Deployment research for regulated jurisdictions

---

## Consequences

### Pure QASH repo (`corp0ratestarts-cmd/pure-qash`)

- Owns its own `GENESIS_CONSTANTS.toml` (separate from umbrella)
- Owns its own proof corpus, CI workflows, claim register, and release path
- Has no Class IV disclosure path
- Has no regulated disclosure features
- Has no user graph evidence retention
- Has no priority fee or fee auction mechanism
- Is NOT a feature toggle or build-flag variant of the umbrella repo
- Runs its own absence guards that fail if any regulated/disclosure concept appears

### Umbrella repo (`corp0ratestarts-cmd/qash`)

- Retains Class IV observer class in its privacy model
- Retains regulated receipt disclosure scaffolding
- Retains compliance documentation that implies structured evidence retention
- Retains sovereign-hardened profile research
- Is NOT modified to accommodate Pure QASH constants or Pure QASH privacy restrictions

### Import policy

Pure QASH may import code and proofs from the umbrella repo under the following rules:

1. Each import must be reviewed as a Pure QASH PR.
2. Imported files must pass Pure QASH absence guards after import.
3. The import origin (commit SHA from umbrella) must be recorded in
   `docs/release/import_manifest.md`.
4. Pure QASH does NOT automatically track umbrella QASH. Any future sync from
   the umbrella must be treated as a new import PR subject to the same review.

### No upstream drift

Pure QASH must not silently inherit regulated-profile concepts via convenience imports.
The CLAUDE.md and CI absence guards are the enforcement mechanism.

---

## Acceptance Checklist

- [x] ADR states that Pure QASH has no Class IV disclosure path
- [x] ADR states that regulated/disclosure features remain in the umbrella repo only
- [x] ADR states that Pure QASH is not a feature toggle inside the umbrella repo
- [x] ADR states that Pure QASH has its own genesis constants and release path
- [x] ADR states that Pure QASH does not automatically track umbrella QASH

---

## Related

- `docs/spec/19_profile_taxonomy.md` — profile definitions
- `ADR-010` — zero-persistence Domain B
- `ADR-011` — trustless genesis and local opsec
