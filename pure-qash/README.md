# Pure QASH — Separate Repository

Pure QASH has been split into its own repository per [ADR-015](../docs/adr/ADR-015-pure-qash-repository-split.md).

**Pure QASH repository:** `corp0ratestarts-cmd/pure-qash`  
**Split decision:** ADR-015, implemented in umbrella PRs #232, #234, #235  
**Final staged SHA (merged to umbrella main):** `76e907187b6334cfd7bd4a816fc2c022af57ebdb`

## What belongs here (umbrella `qash`)

- Regulated Profile: Class IV observer, disclosure key, lawful-basis flows
- Sovereign Hardened Profile: attested DPU/TEE/HSM boundary
- Compliance artifacts and jurisdiction-specific evidence
- Production networking, threshold signing, ZK verifier integration
- Profile taxonomy and boundary enforcement

## What does NOT belong here

- Pure QASH consensus implementation (`crates/` in pure-qash)
- Pure QASH genesis constants
- Pure QASH absence guards
- Pure QASH CI workflows
- Pure QASH privacy model (no-Class-IV, no-disclosure-key)

## Profile boundary rule

The umbrella repo MUST NOT claim Pure QASH-only privacy properties.
Any release from this repo is Regulated-capable by default.
See `scripts/check_profile_boundary.sh` and `docs/spec/19_profile_taxonomy.md`.
