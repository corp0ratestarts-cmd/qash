# Incident Response Runbook

**Date:** 2026-05-30

---

## Scope

QASH protocol repository — vulnerability disclosures, supply chain incidents, cryptographic failures.

---

## Severity Definitions

| Severity | Examples |
|----------|---------|
| **Critical** | Domain A undefined behaviour, genesis hash forgery, cryptographic primitive failure |
| **High** | Supply chain compromise, key material exposure, Domain A/B boundary violation |
| **Medium** | Advisory CVE in dependency, documentation overclaim |
| **Low** | Style/lint issue, advisory-only CI failure |

---

## Response Phases

### Phase 1 — Triage (within 24 hours)

1. Identify severity (Critical / High / Medium / Low) using the table above.
2. Assign an owner.
3. Determine whether genesis-locked state is affected (check `GENESIS_CONSTANTS.toml` and `spec/genesis-artifacts.txt`).

### Phase 2 — Notify (within 48 hours)

- **Critical or High:** File a GitHub Security Advisory draft in the repository Security tab.
- **Supply chain incident:** Open a cargo-deny exception PR (`deny.toml` or `osv-ignore.toml`) with justification.
- Notify relevant maintainers via the contact in `SECURITY.md`.

### Phase 3 — Remediate

1. Develop patch on a dedicated branch.
2. Open a PR; all blocking CI jobs must be green before merge.
3. For genesis-affecting issues, include `[genesis-change-acknowledged]` in the PR description; the `genesis-change-guard` CI job must pass.
4. Tag the fix release if the severity is Critical or High.

### Phase 4 — Post-mortem

1. Document root cause and timeline.
2. Add a regression test covering the failure mode.
3. Update `osv-ignore.toml` or `deny.toml` if the issue involved a dependency advisory.
4. Update `proofs/COVERAGE.md` if any proof obligation is added or modified as a result.
