# QASH Release Baseline Decision

**Date:** 2026-05-26  
**Status:** Authoritative — this document records the release-tag and baseline boundaries.

---

## Release Split

| Tag / Label | Commit | PR boundary | Contents |
|-------------|--------|-------------|----------|
| `qash-pilot-baseline-v0.2.1` | `04ad39d` | Post-PR #168, pre-PR #169 | Pilot execution readiness docs, evidence manifest, pilot package, funding docs, assurance docs; Phase 2-R micro-fix (PR #167) |
| `qash-pilot-baseline-v0.3` | `67665e4` | Post-PR #169 (current `main`) | All of v0.2.1 plus v0.3 multi-operator import/replay with labelled import tracking |
| `v1.0-reference` | TBD | Genesis lock | Deferred until all evidence gates in `ROADMAP.md` are complete and owner sign-off is explicit |

**Decision rationale:**
- PR #169 ("v0.3: multi-operator import/replay") is a clearly scoped feature that advances the MVP vault
  beyond the pilot execution baseline. Folding it into v0.2.1 would blur the pilot evidence boundary.
- `qash-pilot-baseline-v0.2.1` is therefore the last commit before PR #169 merged (`04ad39d`).
- PR #169 defines the `v0.3` baseline (`67665e4`).
- Neither tag is a genesis lock. Genesis lock (`v1.0-reference`) requires the full evidence gate
  in `ROADMAP.md` and is explicitly deferred.

---

## Tagging Instructions

These tags should be created only after confirming that the target commit is clean
(all CI green, evidence captured):

```bash
# v0.2.1 pilot baseline (PR #168 merge, pre-PR #169)
git tag -a qash-pilot-baseline-v0.2.1 04ad39d \
  -m "Pilot execution readiness baseline (post-PR-#168, pre-PR-#169)"

# v0.3 baseline (current main, post-PR #169)
git tag -a qash-pilot-baseline-v0.3 67665e4 \
  -m "v0.3 multi-operator import/replay baseline"

# Push tags (after confirming CI on the target commits)
git push origin qash-pilot-baseline-v0.2.1
git push origin qash-pilot-baseline-v0.3
```

**Do NOT run `git tag v1.0-reference`** until the genesis lock gate in `ROADMAP.md` is complete
and an explicit owner sign-off is recorded.

---

## Evidence Capture

Before creating the tags, capture pre-genesis evidence at each baseline commit:

```bash
# For v0.2.1 evidence
git checkout 04ad39d
bash scripts/capture_pre_genesis_evidence.sh
# Commit only the evidence manifest/index under artifacts/evidence/

# For v0.3 evidence
git checkout 67665e4  # or main
bash scripts/capture_pre_genesis_evidence.sh
```

Raw artifact files that are gitignored should be retained as CI artifacts (365-day retention)
or as local archives. Only the manifest/index file (`artifacts/evidence/manifest.json` or
equivalent) is committed to the repository.
