# TRL 5 Evidence Matrix — QASH

**Current TRL:** 3–4 (working demonstrator in lab environment)  
**Target TRL:** 5 (technology validated in relevant environment)

This matrix tracks the evidence required to justify a TRL 5 claim.

---

## TRL 3–4 Evidence (Current)

| Requirement | Evidence | Status |
|---|---|---|
| Core technology demonstrated in laboratory | `bash scripts/run_mvp_demo.sh --clean` runs end-to-end | ✅ Complete |
| Deterministic output verified across platforms | CI cross-verify: x86-64, AArch64, RISC-V | ✅ Complete |
| Privacy boundary verified programmatically | `build_pilot_evidence_bundle.sh` privacy checks | ✅ Complete |
| Formal proof: stability | `proofs/contractivity/lyapunov_stability.v` | ✅ Complete |
| Formal proof: safety | `proofs/safety/absorbing_halt.v` | ✅ Complete |
| Supply chain controls | OSV scan, SBOM, cargo-deny on every PR | ✅ Complete |
| Independent replay verified | `replay_public_export_bytes` unit tests | ✅ Complete |

---

## TRL 5 Gap Analysis

| Requirement | Current Status | Gap |
|---|---|---|
| Validated in a relevant environment | Lab only | Requires partner pilot with real operator workflow |
| Post-quantum signatures verified (not carried opaquely) | Opaque in Domain A | Dilithium5 + SLH-DSA integration in Domain B |
| Hardware attestation integrated | Stub only (`UnimplementedAttestationGate`) | TPM 2.0 integration in PAL |
| Multi-run stability with real incident data | Synthetic only | Partner pilot with notional incident scenarios |
| Independent third-party replay verification | Self-verified | External partner must replay independently |
| Threat model review | Internal only | Independent security review |
| Formal proof: cascade (TH-9/TH-10/TH-11) | In progress (`proofs/cascade/`) | Complete Coq proofs |

---

## TRL 5 Target Evidence Plan

| Evidence Item | Owner | Target Date |
|---|---|---|
| Partner pilot completion report | Lead + partner | [DATE] |
| Partner independent replay verification | Partner | [DATE] |
| Dilithium5 Domain B integration + tests | Lead | [DATE] |
| TPM 2.0 attestation PAL stub → real | Lead | [DATE] |
| Independent security review report | External reviewer | [DATE] |
| TH-9/TH-10/TH-11 Coq proofs complete | Lead | [DATE] |
| Regulatory mapping (NIS2/DORA/NCSC CAF) | Lead | [DATE] |

---

## TRL 5 Claim Statement (Draft)

> "QASH has been validated in a relevant operational environment. A partner operator has successfully issued offline incident receipts, exported a public commitment transcript, and independently verified the deterministic replay root on a separate machine. No private incident body appeared in any public artifact. The validation used notional incident data representative of real operational scenarios."

This statement becomes claimable once all TRL 5 gap items above are closed.

All claims are governed by `docs/mvp/claims_register.md`.
