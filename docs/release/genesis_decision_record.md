# Genesis Decision Record

**PR:** #240
**Date:** pending owner decision
**Branch:** `claude/genesis-decision-240`

## Owner Decision

Choose exactly one outcome and record it below:

- [ ] A. genesis-candidate
- [ ] B. RC-only milestone
- [ ] C. defer

**Chosen outcome:** _(fill in)_

---

## Outcome A — Genesis-candidate

Only valid if the PR body contains `[genesis-change-acknowledged]`.

If Outcome A is chosen, record:

```
Owner selected genesis-candidate.
[genesis-change-acknowledged] was present in the PR body.
deployment_authoritative remains false pending production deployment sign-off.
```

Note: genesis-candidate fixes the candidate artifact set for review; it is not
production deployment authorization. `deployment_authoritative` stays `false`
until a separate production deployment sign-off occurs.

Changes made under Outcome A:
- `GENESIS_CONSTANTS.toml`: `genesis_status = "genesis-candidate"`, `deployment_authoritative = false`
- Genesis hash recomputed via `cargo run --bin genesis-hash`
- `spec/genesis-artifacts.txt`: pre-lock caveat removed; final genesis hash recorded
- `docs/release/pre_genesis_evidence_snapshot.md`: status updated to genesis-candidate

---

## Outcome B — RC-only milestone

No genesis constants changed.

If Outcome B is chosen, record:

```
Owner selected RC-only milestone.
Rationale: (fill in)
GENESIS_CONSTANTS.toml: unchanged (genesis_status = "provisional", deployment_authoritative = false)
```

Changes made under Outcome B:
- `docs/release/pre_genesis_evidence_snapshot.md`: RC-only decision noted with date
- No changes to `GENESIS_CONSTANTS.toml` or `spec/genesis-artifacts.txt`

---

## Outcome C — Defer

No genesis constants changed.

If Outcome C is chosen, record:

```
Owner selected defer.
Blocker(s): (fill in)
Required follow-up PRs: (fill in)
GENESIS_CONSTANTS.toml: unchanged (genesis_status = "provisional", deployment_authoritative = false)
```

Changes made under Outcome C:
- `docs/release/pre_genesis_evidence_snapshot.md`: deferral and next-gate conditions recorded
- No changes to `GENESIS_CONSTANTS.toml` or `spec/genesis-artifacts.txt`
- No `v1.0-reference` tag

---

## Evidence Reviewed

Before recording the decision, confirm each item:

- [ ] `docs/traceability.md` — zero provisional citation notices
- [ ] `docs/release/v1_axiom_boundary.md` — all axioms classified
- [ ] `docs/release/coq_rust_parity.md` — 12 vectors (TV-0..TV-11)
- [ ] `docs/audit/domain_b_stub_register.md` — every stub has a disposition
- [ ] `crates/pal/src/receipt.rs` — ChaCha20-Poly1305 AEAD; zero XOR in production path
- [ ] `GENESIS_CONSTANTS.toml` — `genesis_status = "provisional"`, `deployment_authoritative = false` before decision
- [ ] `spec/genesis-artifacts.txt` — PDF SHA-256 recorded; pre-lock caveat present (before Outcome A only)
- [ ] `bash scripts/verify_genesis_hash.sh` — exits 0 in provisional mode
