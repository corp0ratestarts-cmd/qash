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

| File | Status | One-sentence decision | Traceability |
|------|--------|----------------------|-------------|
| `ADR-001-phi-safety-accumulator.md` | Accepted | Φ_safety is a sum (not max) over validator slash accumulators; `PHI_MAX_SAFE = 500_000_000` raw; breach triggers H7 absorbing halt. | P0-1, P0-9 |
| `ADR-002-phi-safety-aggregation.md` | Accepted | Confirms sum-aggregation rule for Φ_safety; narrows boundary to threshold check before commit point. | P0-1 |
| `ADR-003-state-root-and-encoding.md` | Accepted | v1.0 state root uses `H_domain(STATE_ROOT, Encode_for_commitment(…))` (SHA3-256 over `tag_u32_le ‖ input`); cascade is not active for v1.0 state roots. | P0-2, P0-9 |
| `ADR-004-absorbing-halt-layering.md` | Accepted | Domain A emits halt reason codes; PAL reads them via `Halt::absorbing_reset()` and is solely responsible for zeroization, scheduler, and watchdog. | P0-5 |
| `ADR-005-rust-toolchain-version.md` | Accepted | Toolchain pinned in `rust-toolchain.toml`; `stable` channel; no nightly in Domain A. | P0-8 |
| `ADR-006-runtime-optimization-track.md` | Proposed | PR #93 runtime optimization work is consensus-byte-preserving only; gates defined in `adr006_phase2r_evidence.md`. | — |
| `ADR-007-uc-mja-cascade-track.md` | Proposed | UC-MJA cascade research track; not a genesis-blocking item. | — |
| `ADR-008-sovereign-storage-tiers.md` | Proposed | Defines Domain B sovereign storage tier model. | — |
| `ADR-009-domain-b-indexing-and-zk-prover-sizing.md` | Proposed | Domain B ZK prover sizing and indexing strategy; not a v1.0 genesis item. | — |
| `ADR-010-zero-persistence-domain-b.md` | Proposed | Zero-persistence Domain B admission rule; all sensitive material zeroized before halt. | — |
| `ADR-011-trustless-genesis-local-opsec.md` | Proposed | Trustless genesis and vendor-agnostic local hardware OpSec model. | — |
| `ADR-012-streaming-state-root-encoding.md` | Accepted | Streaming canonical state encoding removes intermediate heap allocation; byte-identical to batch path. | P0-2 |
| `0001-domain-isolation.md` | Accepted | Domain A / Domain B isolation is protocol law; cross-domain value flow is a protocol violation. | — |
| `0002-transition-safe-fixed-point.md` | Accepted | All fixed-point arithmetic in Domain A uses checked operations; overflow → absorbing halt. | — |
| `IC-001-no-heap-cascade.md` | Proposed | Cascade verification must not allocate on the heap; stack-resident buffers only. | — |

## Superseded Variants (history only)

| File | Superseded by |
|------|--------------|
| `ADR-001-phi-safety-and-threshold.md` | `ADR-001-phi-safety-accumulator.md` |
| `ADR-003-state-root-encoding.md` | `ADR-003-state-root-and-encoding.md` |
| `ADR-004-halt-layering-domain-a-vs-pal.md` | `ADR-004-absorbing-halt-layering.md` |
