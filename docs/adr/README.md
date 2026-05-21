# Architecture Decision Records and Implementation Constraints

ADRs document engineering decisions that either:

- fill a PDF-silent gap,
- choose between implementation strategies permitted by the PDF, or
- define layer boundaries needed to implement a PDF requirement.

Implementation constraints (`IC-*`) document how illustrative PDF pseudocode is
translated into Domain A-safe code without changing the PDF requirement.

Every ADR or IC must reference either a verbatim PDF quote or explicitly state
`PDF-SILENT`.

## Active ADR Index

| ADR | Status | Purpose |
|-----|--------|---------|
| `ADR-006-runtime-optimization-track.md` | Proposed | Schedules the PR #93 runtime optimization track and its consensus-byte-preservation gates. |
