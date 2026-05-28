# ✅ Pre-Genesis Full-Repo Audit — PASS

**Commit:** `c7f4fb3bff4b1dd24d7553b3d40d88dfa26b1e0a`
**Timestamp:** 2026-05-28T17:51:45Z
**Overall verdict:** **PASS**

---

## Blocking phase verdicts

| Phase | Script | Verdict |
|-------|--------|---------|
| Phase 9 — Claim boundary | `audit_claim_boundary.sh` | ✅ PASS |
| Phase 10 — Domain A/B boundary | `audit_domain_boundary_full.sh` | ✅ PASS |
| Phase 2 — Rust bad practices | `audit_rust_bad_practices.sh` | ✅ PASS |
| Phase 6 — Panic surface | `audit_panic_surface.sh` | ✅ PASS |
| Phase 4 — Unsafe boundary | `audit_unsafe_boundary.sh` | ✅ PASS |
| Phase 5 — Liveness loops | `audit_liveness_loops.sh` | ✅ PASS |

## File inventory

| Metric | Value |
|--------|-------|
| Total tracked files | 362 |
| Domain A files (`crates/consensus/src/`) | 21 |
| Domain B files (`pal/address/model/src/`) | 29 |
| Workspace packages | 5 workspace packages |

## Proof status

| Metric | Value |
|--------|-------|
| Coq proof files | 26 .v files present |
| Open unsafe exceptions | 1 |

## Advisory phase summaries

### Phase 3 — Strict Clippy
_See `artifacts/audit/strict_clippy.txt` for full output._

Clippy warnings: 2429

### Phase 7 — Concurrency patterns
_See `artifacts/audit/concurrency_patterns.md`._

Lock-across-await candidates: 0

## Dependency risk

Open dependency risk entries: 0

_See `docs/audit/dependency_risk_register.md` for triage status._

## Genesis-lock gate

✅ All blocking phases pass. This commit is eligible for genesis-lock
subject to: dependency risk triage complete, all advisory findings
triaged with documented decisions, and exception register reviewed.

---

## Phase report index

- [`claim_boundary.md`](./claim_boundary.md)
- [`concurrency_patterns.md`](./concurrency_patterns.md)
- [`domain_boundary_full.md`](./domain_boundary_full.md)
- [`file_inventory.md`](./file_inventory.md)
- [`liveness_loops.md`](./liveness_loops.md)
- [`panic_surface.md`](./panic_surface.md)
- [`rust_bad_practices.md`](./rust_bad_practices.md)
- [`unsafe_boundary.md`](./unsafe_boundary.md)
