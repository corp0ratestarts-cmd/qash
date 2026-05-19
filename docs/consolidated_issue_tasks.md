# Consolidated Issue Tasks

This document tracks four concrete follow-up tasks identified during repository review.

## 1) Typo / Editorial Cleanup

**Title:** Remove accidental empty fenced code block in root README

**Problem:** `README.md` contains an empty fenced code block with no content, which appears to be accidental formatting noise.

**Scope:**
- Remove the empty code fence.
- Verify markdown rendering around the surrounding section remains unchanged.

**Acceptance Criteria:**
- `README.md` no longer contains the empty code fence.
- The rendered Repository Structure section is unchanged other than removal of blank code block.

---

## 2) Bug Fix

**Title:** Prevent false-green `cargo test` runs at workspace root

**Problem:** Running tests from repository root can report passing output with zero tests executed in multiple targets, which can mask regressions.

**Scope:**
- Configure workspace test defaults (e.g., `default-members`) or add a canonical root test command/script that explicitly runs meaningful crate test targets.
- Update contributor docs with the canonical command.

**Acceptance Criteria:**
- Root-level recommended test command executes non-zero meaningful tests (or explicitly verifies expected suites).
- CI/local docs clearly specify the canonical command.

---

## 3) Documentation Discrepancy

**Title:** Align README authority narrative with pre-lock PDF status

**Problem:** Top-level docs can be read as if normative PDF authority is fully active, while `spec/pdf/README.md` states the repository remains pre-lock until `QASH_Spec_v1.0.pdf` is committed.

**Scope:**
- Add explicit pre-lock caveat in root `README.md` near repository structure / authority sections.
- Ensure wording is consistent with `spec/pdf/README.md`.

**Acceptance Criteria:**
- README clearly states current pre-lock status.
- README and `spec/pdf/README.md` do not conflict on normative authority status.

---

## 4) Test Improvement

**Title:** Add CI-visible coverage for vector integrity beyond ignored generators

**Problem:** Vector generation helper tests are ignored/manual, making drift harder to catch automatically.

**Scope:**
- Add a non-ignored smoke test that validates checked-in vectors exist and meet structural invariants.
- Keep heavyweight regeneration helper ignored.

**Acceptance Criteria:**
- CI executes at least one non-ignored vector integrity test.
- Test fails on missing/malformed checked-in vectors.

