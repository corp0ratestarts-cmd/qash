# Post-Merge MVP Baseline Audit

**Date:** 2026-05-25  
**Scope:** MVP incident receipt demonstrator after `mvp-incident-receipt-v0.1` and subsequent `main` state.  
**Baseline tag:** `mvp-incident-receipt-v0.1` → `c31776c4022e23f0d3204de0e211b3f9071dee5f`.  
**Current main during audit:** `b1e1f8ec8719ab1e802faf9544d93493331c69ff`.

## Summary

The MVP remains bounded as a local Domain B demonstrator. The audit found no evidence that `TX-MVP-ReceiptCommit` is admitted into Domain A consensus, genesis constants, or `advance_epoch`.

The public transcript path remains commitment-only. Private receipt bodies are stored under the local vault and appear only in selected disclosure outputs. Imported public commitments can be replayed but cannot be disclosed because the importing workspace does not hold private receipt bodies.

## Audited files

- `src/main.rs`
- `src/demo.rs`
- `crates/pal/src/mvp.rs`
- `crates/pal/src/mvp_vault.rs`
- `scripts/run_mvp_demo.sh`
- `docs/mvp/claims_register.md`
- `docs/funding/*`

## Findings

### Domain boundary

**Status:** Pass

- `src/main.rs` routes `qash demo ...` to the local demo CLI before the existing health-demo path.
- `TX-MVP-ReceiptCommit` remains in `crates/pal/src/mvp.rs`, documented as Domain B demonstrator material and not as a genesis-admitted Domain A transaction.
- Repository search for `TX-MVP-ReceiptCommit`, `advance_epoch`, and `GENESIS_CONSTANTS` did not show MVP admission into the Domain A transition path.

### Public transcript and privacy boundary

**Status:** Pass

- `sync` exports public commitment records.
- `replay` folds public exports into a deterministic root and optional JSON report.
- Replay report fields are limited to profile, record count, commitment root, public-transcript flag, private-payload flag, and status.
- `scripts/run_mvp_demo.sh` asserts that public commitments do not contain the private body strings used in the demo.

### Selective disclosure

**Status:** Pass

- `disclose` requires a local private receipt body.
- Import-side sync stores public commitments only.
- Tests assert that an imported-only workspace can replay imported commitments but cannot disclose the corresponding receipt body.

### WAL and import validation

**Status:** Pass with future hardening recommended

- WAL magic, record magic, truncation, and public-export mismatch checks fail closed.
- Public commitment import validates the header and fixed record length before persisting the import.
- Future hardening should add schema-version fields and structured error codes for public import artifacts.

### CodeQL-sensitive fixture patterns

**Status:** Pass with watch item

- MVP receipt fixtures use enum-selected deterministic bytes rather than hardcoded byte arrays or numeric fixture salts.
- Existing tests still use synthetic body text to assert leakage boundaries. This is acceptable because these strings are payload leak sentinels, not secret keys or nonce fixtures.

### Claims boundary

**Status:** Pass

- `docs/mvp/claims_register.md` correctly states allowed and blocked MVP claims.
- Funding documents should continue to treat the claims register as governing language.

## Follow-up items

1. Reference `docs/mvp/claims_register.md` from the top-level README and ROADMAP.
2. Add JSON schema validation for replay reports.
3. Add a deterministic fixture pack under `examples/mvp/`.
4. Add benchmark-lite evidence for MVP replay/import/disclosure timings.
5. Add an operator runbook for local demo execution and artifact interpretation.
6. Add a passive-observability threat model for public commitments and replay reports.
7. Add ADR-006 Phase 2-R evidence notes because current `main` includes consensus-path runtime optimizations after the clean MVP tag.

## Audit conclusion

The MVP baseline is suitable for bounded funding and pilot discussions using the claims register language. It should not be described as a production payment system, settlement rail, identity system, production ZK verifier, hardware attestation system, or genesis-admitted transaction class.
