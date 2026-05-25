# Risk Register — QASH Phase 1 Research Programme

**Scope:** Offline incident-receipt commit demonstrator (Domain B MVP)  
**Last updated:** 2026-05-25

---

## Risk table

| ID | Risk | Likelihood | Impact | Severity | Mitigation |
|----|------|-----------|--------|----------|------------|
| R1 | Security review identifies P0/P1 vulnerability in WAL format or commitment scheme | Medium | High | **High** | WP1 external review planned for months 1–3; WAL format is fixed-size with magic headers, reducing attack surface; existing fuzz corpus covers truncation and corruption cases |
| R2 | Partner organisation is unable to provide representative incident data for WP2 pilot | Medium | Medium | **Medium** | Two fallback partners identified in outreach pipeline; synthetic log corpus can substitute for TRL 5 validation if necessary |
| R3 | Cross-ISA determinism breaks under a new compiler or toolchain update | Low | High | **Medium** | Pinned Rust toolchain (`rust-toolchain.toml`); CI gates on x86-64/AArch64/RISC-V replay-root equality; any divergence halts CI immediately |
| R4 | Selective disclosure mechanism is found to leak correlation information via side-channels (timing, record size) | Low | High | **Medium** | WP1 constant-time audit; commitment sizes are fixed (140 bytes); export format pads to fixed record size |
| R5 | Scope creep into Domain A (production consensus) claims during partner engagement | Medium | High | **High** | `docs/mvp/claims_register.md` governs all communications; all partner outreach templates use blocked-claim language from the register; CI check enforces no Domain B→A contamination |
| R6 | Regulatory classification of the demonstrator as a financial instrument | Low | High | **Medium** | No payment, settlement, or token functionality is present or planned; claim boundary explicitly excludes these; legal review recommended before any commercial pilot |
| R7 | Performance is insufficient for partner's incident log volume at TRL 5 | Low | Medium | **Low** | WP2.4 performance characterisation at 10k/100k records; WAL append is O(1); replay is a single-pass hash fold — throughput is I/O-bound, not compute-bound |
| R8 | Key personnel unavailability during WP1–WP2 overlap (months 2–4) | Low | Medium | **Low** | WP1 external reviewer is independent; WP2 tasks are documented in sufficient detail for handover |
| R9 | Fuzz campaign (WP1.3) exposes parser crash with no straightforward fix | Low | High | **Medium** | WAL parser has no recursion and processes fixed-size records only; worst case is a WAL truncation detection path, which already triggers a non-fatal error |
| R10 | Funding programme requires open publication of partner incident data | Low | High | **Medium** | Demonstrator is designed for anonymised synthetic data at pilot stage; real incident data is never required; public transcript contains only commitments |

---

## Severity definitions

| Severity | Likelihood × Impact |
|----------|---------------------|
| Critical | High × High |
| High | Medium × High or High × Medium |
| Medium | Low × High, Medium × Medium, or High × Low |
| Low | Low × Medium or Low × Low |

---

## Claim boundary

This risk register applies only to the Domain B MVP demonstrator. Risks relating to Domain A production consensus, genesis admission, or network deployment are out of scope. Consult `docs/mvp/claims_register.md` for the full claim boundary.
