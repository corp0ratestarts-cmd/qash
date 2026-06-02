# ADR-014: Language Boundary Policy

**Status:** Accepted
**Date:** 2026-06-02
**Replaces:** Informal per-crate language assumptions

---

## Context

QASH spans multiple languages. Without an explicit policy, language choices can
drift — OCaml snippets appear in tooling, Python creeps into scripts, mobile adapters
generate Kotlin/Swift at the boundary. This ADR records the authoritative language
assignment so auditors and contributors have a single reference.

---

## Decision

### Domain A — Deterministic Consensus Core

**Language:** Rust only (`crates/consensus/`).

No other language may appear in Domain A. The constraints (no `unsafe`, no float,
no `usize` in arithmetic, checked overflow, `no_std`/no-alloc) are enforced by CI
tripwires. Any future Domain A code must be Rust.

### PAL Core — Platform Abstraction Layer

**Language:** Rust only (`crates/pal/`).

The PAL is Domain B and permits `unsafe` under audit, but the implementation
language remains Rust. Hardware vendor SDKs that provide only a C ABI must be
wrapped in a thin, safe Rust binding — the raw C ABI may not be called from
within `crates/pal/` directly.

### Formal Proofs

**Language:** Coq/Rocq (`proofs/`).

All deductive proof obligations (safety, liveness, cryptographic reduction
statements) must be in Coq. Zero `Admitted` beyond the documented axioms
(AX-1, AX-2, AX-3) in any proof that is compiled by CI.

### Extracted Reference Oracle (optional, non-authoritative)

**Language:** OCaml (`model/`), extracted from Coq via `Extraction`.

The extracted OCaml oracle is non-authoritative for protocol state roots; it is
a reference oracle for observational equivalence testing only. It may not be
imported into Domain A or PAL. Any divergence between OCaml oracle output and Rust
output is an AX2_rust_refinement violation that blocks merge.

### Hardware / Vendor Boundary

**Language:** Thin C ABI behind a safe Rust wrapper.

Where a hardware vendor SDK provides only a C ABI (TPM 2.0, TDX, SEV-SNP, etc.),
the integration must:
1. Expose a safe Rust wrapper in `src/hardware/` or `crates/pal/src/`.
2. Mark the unsafe `extern "C"` block with an `// SAFETY: ...` comment.
3. Register the stub in `docs/audit/domain_b_stub_register.md`.

### Mobile Transport Adapters

**Language:** Swift (iOS) or Kotlin/JVM (Android) — outside the protocol core only.

Mobile transport adapters may use Swift or Kotlin for platform API access, but:
- They must communicate with the protocol core via a stable C ABI or Protobuf
  boundary.
- No Swift/Kotlin code may express consensus logic, state transitions, or key
  derivation — these must live in a Rust binary compiled to a shared library.

### Audit / Evidence Tooling

**Language:** Rust xtask (`xtask/`) preferred.

New evidence-capture, boundary-audit, and proof-hash commands should be written
as `cargo xtask` subcommands. Python is permitted for report generation and
benchmark plot scripts only, not for protocol-adjacent logic. Bash scripts remain
as thin wrappers around xtask commands for CI compatibility.

---

## Rationale

- **Rust everywhere protocol-adjacent** eliminates language-mixing bugs at
  protocol boundaries and keeps the security surface auditable.
- **Coq for proofs** is already established and required by CI; this ADR
  makes the exclusive claim explicit.
- **Extracted OCaml as non-authoritative oracle** preserves the Coq extraction
  toolchain without granting the oracle normative status.
- **C ABI only at hardware vendor boundary** isolates `unsafe` to the thinnest
  possible layer and keeps it visible in the audit register.
- **xtask over freestanding scripts** means evidence tooling is type-checked,
  gets CI coverage, and can be depended on from other Rust code.

---

## Consequences

- All new tooling PRs must add xtask subcommands for non-trivial operations.
- Existing Bash scripts are kept for backwards compatibility but are wrappers.
- Any language not listed above requires a new ADR before use.
- Mobile transport adapters that violate the C ABI boundary policy block merge.
