# Unsafe Code Exception Register

This file documents every `unsafe` block or function in Domain B (`qash-pal`, `qash-address`, `model`, `src/`) that is accepted as a justified exception to the general "prefer safe code" guideline.

**Domain A (`qash-consensus`) has `#![forbid(unsafe_code)]` and zero tolerance for `unsafe`. No exceptions are ever recorded here for Domain A.**

For Domain B, every `unsafe` block or function must satisfy **one** of:
1. A `// SAFETY: <explanation>` comment immediately before the block (preferred), OR
2. An entry in this register with owner sign-off.

If neither is present, `audit_unsafe_boundary.sh` records it as an advisory finding requiring triage.

---

## Register format

Each entry:

```markdown
### <crate>/<file>:<line-range>

**Pattern:** `unsafe { … }` / `unsafe fn` / `extern "C"` / etc.
**Justification:** Why unsafe is necessary and cannot be replaced with safe code.
**Alternatives considered:** What safe alternatives were evaluated and why they were rejected.
**Bounded invariant:** What invariant the caller must uphold for soundness.
**Owner:** @<github-handle>
**Date:** YYYY-MM-DD
**Review status:** Pending / Accepted / Scheduled-for-removal
```

---

## Current entries

*No exceptions registered. All Domain B `unsafe` code must carry a `// SAFETY:` comment. Use this register only for cases where a comment is architecturally impractical (e.g., generated code, macro-expanded sites).*

---

## Audit history

| Date | Auditor | Scope | Findings | Action |
|------|---------|-------|----------|---------|
| *(none yet)* | | | | |
