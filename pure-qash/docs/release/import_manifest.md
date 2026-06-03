# Pure QASH Import Manifest

**Purpose:** Auditable record of all code imported from `corp0ratestarts-cmd/qash` (umbrella).

Per ADR-015 and `docs/spec/19_profile_taxonomy.md` in the umbrella repo, Pure QASH does not
automatically track umbrella QASH. Every import must be recorded here with:
- Source commit SHA from umbrella
- Files imported
- Files explicitly excluded and why
- Date reviewed
- Absence guard result

---

## Import 0 — Initial scaffold

**Date:** 2026-06-03  
**Source repo:** `corp0ratestarts-cmd/qash`  
**Source commit SHA:** *(to be filled in when import is executed)*

### Files imported

| File | Source path | Notes |
|------|-------------|-------|
| `crates/consensus/src/*.rs` | `crates/consensus/src/*.rs` | Full import; no regulated content |
| `crates/pal/src/privacy/public_transcript.rs` | same | Root-only transcript boundary |
| `crates/pal/src/privacy/erasure.rs` | same | Zeroization primitives |
| `proofs/contractivity/` | same | TH-1, TH-2, TH-3 Lyapunov proofs |
| `proofs/safety/absorbing_halt.v` | same | TH-4/5/6/8 halt proofs |
| `proofs/composition/th3_system_closure.v` | same | TH-3 system closure |
| `proofs/cascade/` | same | Cascade determinism and health proofs |
| `proofs/ordering/causal_ordering.v` | same | CO-1..CO-5 sort key proofs |
| `proofs/util/` | same | List injectivity lemmas |
| `proofs/model/` | same | Formal model and refinement axiom |
| `docs/spec/00_execution_model.md` | same | Domain A/B substrate law |
| `docs/spec/01_consensus.md` | same | State space and transition |
| `docs/spec/02_transition_axioms.md` | same | A0–A11 axioms |
| `docs/spec/03_transactions.md` | same | TX-0/TX-1 semantics (basis only) |
| `docs/spec/14_zero_persistence_pipeline.md` | same | Zero-persistence production spec |

### Files explicitly excluded

| File | Reason |
|------|--------|
| `docs/spec/09_privacy_model.md` | Umbrella version contains Class IV; Pure QASH has own file |
| `docs/compliance/` | All files imply regulated evidence retention |
| `docs/assurance/compliance_mapping.md` | References Class IV / lawful disclosure |
| `docs/assurance/dpia.md` | User-activity evidence structures |
| `crates/pal/src/threshold/` | Demo-only threshold signing |
| `crates/pal/src/clone/transport/` | Interface-only transports (not zero-persistence verified) |
| `crates/pal/src/mvp*.rs` | Demo profile with unlocked Domain A |
| `crates/pal/src/zk/plonky3.rs` | Post-v1 production ZK backend |
| `docs/funding/` | Not relevant to Pure QASH protocol |
| `docs/pilot/` | Compliance/pilot evidence structures |
| `src/` (hosted binary) | Umbrella-specific entrypoint |

### Absence guard result

```
scripts/check_pure_absence_guards.sh: PASS
```
*(to be verified after actual import)*

---

*Future imports must add a new numbered section following this template.*
