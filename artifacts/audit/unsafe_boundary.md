# Unsafe Boundary Audit

**Commit:** `7c1d41fd2447b0aedd507e32ad5e9208c16980cc`
**Timestamp:** 2026-05-27T07:04:03Z
**Domain A status:** ✅ PASS
**Domain B missing SAFETY comment:** 0 advisory finding(s)
**Domain B with SAFETY comment:** 0 compliant site(s)

## Policy

**Domain A:** `qash-consensus` has `#![forbid(unsafe_code)]`. Any `unsafe` hit
exits 1 unconditionally. SAFETY comments and exception entries do not override
— Domain A forbids unsafe absolutely.

**Domain B:** Any `unsafe` block or function without a preceding `// SAFETY:`
comment (within 5 lines) AND without an entry in `docs/audit/unsafe_exceptions.md`
→ advisory finding requiring triage before genesis-lock.

**unsafe detection pattern** (precise — skips `forbid`/`deny` attribute lines):
```
unsafe\s*(\{|fn\s|impl\s|trait\s|extern\s)
```

## Domain A results (blocking)

- **Directory:** `crates/consensus/src`

✅ No unsafe found — consistent with `#![forbid(unsafe_code)]`.

## Domain B results (advisory)

✅ No unsafe found in Domain B.

## cargo geiger count summary (advisory)

```
error: no such command: `geiger`

help: view all installed commands with `cargo --list`
help: find a package to install `geiger` with `cargo search cargo-geiger`
(cargo geiger failed or not installed — advisory only)
```

## Verdict

**PASS** — Domain A is clean. Domain B has 0 advisory finding(s) requiring triage.
