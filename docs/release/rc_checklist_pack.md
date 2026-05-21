# Release Candidate Checklist Pack

**Purpose:** This pack defines the mandatory release-candidate (RC) gates and evidence required **before genesis decision**.

**Decision Rule:** Genesis decision is blocked until all P0 gates are green and all required owners have signed off.

For the current pre-genesis integration snapshot, use
`docs/release/pre_genesis_evidence_snapshot.md` before applying this RC checklist.

## 1) P0 Gate Matrix (with evidence links/artifacts)

| Gate ID | P0 Gate | Owner | Status | Evidence / Artifact Link | Notes |
|---|---|---|---|---|---|
| P0-1 | Deterministic consensus replay across canonical vectors | Consensus | ☐ Pending / ☐ Pass / ☐ Fail | `crates/consensus/tests/golden_replay.rs` + CI job URL | Must match expected state roots bit-for-bit. |
| P0-2 | Cross-ISA binary/output equivalence checks complete | Runtime | ☐ Pending / ☐ Pass / ☐ Fail | Cross-ISA report bundle (`artifacts/cross-isa/`) | Includes x86_64 + aarch64 builds and runtime traces. |
| P0-3 | Two-stage/reproducible build verification | Runtime | ☐ Pending / ☐ Pass / ☐ Fail | `scripts/verify_two_stage_build.sh` output + CI URL | Deterministic build checksums required. |
| P0-4 | Coq formal proofs CI fully green | Formal Methods | ☐ Pending / ☐ Pass / ☐ Fail | Coq CI run URL + `proofs/STATUS.md` snapshot | No skipped mandatory proofs. |
| P0-5 | Genesis constants finalization and fingerprint lock | Release | ☐ Pending / ☐ Pass / ☐ Fail | `GENESIS_CONSTANTS.toml` hash + config fingerprint artifact | Hash/fingerprint must match release manifest. |
| P0-6 | Change-freeze policy active and enforced | Release | ☐ Pending / ☐ Pass / ☐ Fail | Freeze announcement URL + exception log | Only critical-fix exceptions allowed. |

---

## 2) Cross-ISA Evidence Artifacts + Exact Toolchain Identifiers

Attach/store all files under: `artifacts/cross-isa/<rc-tag>/`

Required artifacts:

1. `toolchains.txt` with exact identifiers:
   - `rustc --version --verbose`
   - `cargo --version --verbose`
   - `rustup show`
   - target triples enabled (e.g., `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`)
   - linker/compiler versions (`clang --version`, `gcc --version`, `ld --version` as applicable)
2. `build_x86_64.log` and `build_aarch64.log`
3. `checksums.txt` for produced artifacts (SHA-256)
4. `runtime_trace_x86_64.json` and `runtime_trace_aarch64.json`
5. `trace_diff_report.md` proving equivalence (or documenting accepted deltas with approval)
6. `environment_manifest.json` (kernel, container image digest, CPU flags)

Pass criteria:
- Same inputs produce equivalent outputs/traces across ISAs (except explicitly approved non-consensus metadata fields).
- Toolchain identifiers are exact and archived with the RC.

---

## 3) Coq CI Green Run References

Attach/store all files under: `artifacts/coq-ci/<rc-tag>/`

Required references:

- Primary CI green run URL for Coq pipeline.
- Immutable run identifier (run ID / commit SHA).
- Exported job logs for:
  - `proofs/contractivity/lyapunov_stability.v`
  - proof status summary from `proofs/STATUS.md`
- Re-run confirmation (optional but recommended) showing no flakiness.

Pass criteria:
- Coq-required jobs are green on the exact RC commit SHA.
- No unresolved mandatory proofs.

---

## 4) Final Genesis Constants Snapshot Hash + Config Fingerprint

Attach/store all files under: `artifacts/genesis/<rc-tag>/`

Required artifacts:

1. `GENESIS_CONSTANTS.toml` snapshot copy.
2. `genesis_constants.sha256` computed from snapshot.
3. `config_fingerprint.txt` derived from full effective configuration.
4. `fingerprint_method.md` documenting canonicalization + hashing method.

Reference source file:
- `GENESIS_CONSTANTS.toml`

Pass criteria:
- Snapshot hash and config fingerprint are immutable, recorded in release manifest, and signed by required owners.

---

## 5) Performance Claim Evidence

This is conditional evidence, not a P0 genesis prerequisite. It is required for
any RC, README, release note, or certification package that claims latency,
throughput, stack-depth, or tx-heavy admission performance.

Required artifacts:

1. Criterion reports under `artifacts/benchmarks/<rc-tag>/`.
2. Exact command lines and machine/toolchain identifiers.
3. Before/after state-root parity logs for any Phase 2-R runtime optimization.
4. Cross-ISA replay parity for the same vector set used by the benchmarked code.

Pass criteria:
- No performance claim is published without archived benchmark artifacts.
- Phase 2-R changes remain consensus-byte-preserving as required by
  `docs/adr/ADR-006-runtime-optimization-track.md`.

---

## 6) Explicit Change-Freeze Window Policy

**Freeze starts:** RC candidate cut (`T0`).

**Freeze ends:** Genesis decision complete (`T_decision`) or explicit rollback by Release owner.

Policy during freeze window:
- No feature changes.
- No refactors.
- No dependency upgrades unless required for a critical fix.
- **Only critical-fix exceptions are allowed**, defined as:
  1. Safety/security vulnerability,
  2. Consensus correctness risk,
  3. Determinism/reproducibility break,
  4. Production-blocking runtime crash/data corruption.

Exception process:
1. Open “Freeze Exception” ticket with impact analysis and rollback plan.
2. Obtain approvals from Consensus + Runtime + Release (Formal Methods also required if proof surface changes).
3. Merge minimal patch only.
4. Re-run all affected P0 gates and update evidence links.

---

## 7) Mandatory Multi-Owner Sign-off (before genesis decision)

All signatures below are required before genesis decision:

| Role | Owner | Decision | Date (UTC) | Evidence Reviewed |
|---|---|---|---|---|
| Consensus |  | ☐ Approve / ☐ Block |  |  |
| Formal Methods |  | ☐ Approve / ☐ Block |  |  |
| Runtime |  | ☐ Approve / ☐ Block |  |  |
| Release |  | ☐ Approve / ☐ Block |  |  |

**Final rule:** If any required owner blocks or any P0 gate is not pass, genesis decision is **NO-GO**.
