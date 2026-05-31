# Claim Boundary Scan

**Commit:** `3439d5d923b68f79f0798cf04f7b05aef0c7789d`  
**Timestamp:** 2026-05-31T00:22:35Z  
**Status:** ✅ PASS — no violations

## Files scanned

- **General scan:** 162 files (`.md`, `.toml`, `.txt` tracked by git, excluding exempt directories)
- **Excluded:** `docs/mvp/claims_register.md`, `docs/audit/`, `docs/platforms/`, `docs/release/`
- **NOT excluded:** `docs/funding/`, `docs/compliance/`

## Pattern groups

- Compliance/certification overclaim patterns: 25
- Platform overclaim patterns: 12

## Suppression policy

Clearly negative uses and explicit blocked/prohibited/avoid example sections are not treated as live claims.
The narrow allowlist marker remains available for one-off cases.

## Allowlist marker

A line containing `<!-- claim-boundary-allow: <reason> -->` suppresses
that line and the **immediately following line only**. No broader suppression.

## Verdict

**PASS** — all scanned files are within the claim boundary.
