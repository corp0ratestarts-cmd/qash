# QASH v1.0 RC — Implementation Status Matrix

**Date:** 2026-06-03
**Milestone:** Outcome B — RC-only milestone (PR #228)
**Authoritative source:** This file is the single source of truth for what is shipped vs. post-v1.

Cross-references: `docs/audit/domain_b_stub_register.md` (stub details),
`proofs/COVERAGE.md` (proof status), `docs/traceability.md` (PDF traceability).

---

## Status taxonomy

| Label | Meaning |
|-------|----------|
| `✅ ACTIVE V1` | Implemented, tested in CI, in the production consensus path |
| `✅ SELF-TESTED` | Implemented and self-tested; not externally certified |
| `⚠️ INTERFACE-ONLY` | Correct type surface; no production backend; fails closed |
| `⚠️ DEMO-ONLY` | Working demo; must not be used in production |
| `📋 POST-V1` | Explicitly deferred; no v1.0 claim |
| `❌ NOT CLAIMED` | Explicitly out of scope for this release |

---

## Domain A — Consensus Core (`crates/consensus/`)

| Component | Status | Notes |
|-----------|--------|-------|
| Epoch state machine (`transition.rs`) | `✅ ACTIVE V1` | Halt conditions H1–H7, absorbing reset |
| Lyapunov stability functions | `✅ ACTIVE V1` | V_convergence, Φ_safety, Coq-proved TH-3/TH-4/TH-5 |
| Canonical encoding (`encoding.rs`) | `✅ ACTIVE V1` | Injective, total, PDF §3.5 compliant |
| Fixed-point arithmetic | `✅ ACTIVE V1` | i128 intermediate, p=1_000_000, checked overflow |
| Consensus hash cascade | `✅ ACTIVE V1` | SHA3-256 + SM3-256 dual-root |
| State root commitment | `✅ ACTIVE V1` | KAT-pinned TV-0a, PDF §3.5.4 |
| Cross-ISA determinism | `✅ ACTIVE V1` | CI-verified on x86_64 + aarch64 + riscv64gc |
| Absorbing halt | `✅ ACTIVE V1` | Terminal, irreversible, Coq-proved |
| TX-0 (NoOp) and TX-1 (score decrement) | `✅ ACTIVE V1` | PDF §A8, Coq-proved perturbation lemmas |
| Sharded state / EFB roots | `✅ ACTIVE V1` | v1.2 shard struct, EFB root in state |
| Version gating / compatibility window | `✅ ACTIVE V1` | Epoch-based version field |

---

## Domain A — Formal Proofs (`proofs/`)

| Property | Status | Notes |
|----------|--------|-------|
| TH-1/TH-2: Encoding injectivity and totality | `✅ ACTIVE V1` | Coq, zero Admitted beyond AX-1/AX-2 |
| TH-3a/b/c: Lyapunov convergence | `✅ ACTIVE V1` | Coq, CI-compiled |
| TH-4/TH-5/TH-6: Absorbing halt monotonicity | `✅ ACTIVE V1` | Coq, CI-compiled |
| TH-7: Cross-ISA replay invariance | `✅ ACTIVE V1` | CI-verified empirical claim |
| TH-8: Full state root uniqueness | `✅ ACTIVE V1` | Coq, CI-compiled |
| TH-9/TH-10/TH-11: Cascade collision resistance | `✅ ACTIVE V1` | Coq typed reduction axioms, non-vacuous |
| AX-2: Coq↔Rust refinement | `✅ ACTIVE V1` | 12 CI vectors (TV-0..TV-11); AXIOM, not open gap |
| TLA+/Apalache safety invariants | `📋 POST-V1` | Advisory; errata-classified (Wave 2) |

---

## Domain B — PAL Core (`crates/pal/`)

| Component | Status | Notes |
|-----------|--------|-------|
| `hosted::Host` (time/net/attest/halt traits) | `✅ ACTIVE V1` | Std-featured hosted implementation |
| Receipt encryption (ChaCha20-Poly1305 AEAD) | `✅ ACTIVE V1` | XOR stub deleted in Wave 3 |
| PublicTranscript type enforcement | `✅ ACTIVE V1` | Domain B; no transcript data crosses to Domain A |
| AllOf dual-hash / AllOfHashPair32 | `✅ ACTIVE V1` | SHA3-256 + SM3-256 dual-root, KAT-tested |
| Plonky3 shape harness | `⚠️ INTERFACE-ONLY` | Profile-lock tests only; production circuit post-v1 |
| Plonky3 production backend | `📋 POST-V1` | `p3-*` stack wired; QASH circuit acceptance deferred |
| Threshold signing (TALUS) | `⚠️ DEMO-ONLY` | XOR placeholder in `combine_shares()`; not for production |
| Bitsliced NTT (SCA hardening) | `⚠️ INTERFACE-ONLY` | Identity transform under `sca-hardened` feature |
| PQC crypto-agility driver | `⚠️ INTERFACE-ONLY` | Suite-gate logic correct; signing drivers not wired |
| Clone transport (QR/NFC/BLE/WiFiDirect/LoRa/Ultrasonic) | `⚠️ INTERFACE-ONLY` | Correct MTU/name; returns `NotAvailable` |
| Rowhammer hardening (CLFLUSH) | `⚠️ INTERFACE-ONLY` | x86_64 stub; real mitigation needs platform integration |
| SoftTRR / CATT | `📋 POST-V1` | Design complete; not implemented |
| Hancke-Kuhn distance bounding | `📋 POST-V1` | Protocol documented; transport not wired |
| WAL robustness | `✅ ACTIVE V1` | Fuzz-tested, crash-recovery tested |

---

## Hardware Attestation (`src/hardware/`)

| Backend | Status | Notes |
|---------|--------|-------|
| TPM 2.0 | `📋 POST-V1` | All methods `Err(NotAvailable)`; gated by `tpm2` feature |
| Intel TDX | `📋 POST-V1` | All methods `Err(NotAvailable)`; gated by `tdx` feature |
| ARM CCA | `📋 POST-V1` | All methods `Err(NotAvailable)`; gated by `arm-cca` feature |
| AMD SEV-SNP | `📋 POST-V1` | All methods `Err(NotAvailable)`; gated by `sev-snp` feature |
| Software field acceleration | `⚠️ INTERFACE-ONLY` | Returns `NotImplemented`; software fallback automatic |
| Power state manager | `✅ ACTIVE V1` | In-memory Domain B operational state recorder |

---

## Compliance and Certification

| Item | Status | Notes |
|------|--------|-------|
| GDPR DPIA | `✅ SELF-TESTED` | Internal alignment artifact; not a regulatory submission |
| CC EAL4+ target | `✅ SELF-TESTED` | `docs/compliance/cc_security_target.md`; no external evaluation |
| FIPS 140-3 alignment | `✅ SELF-TESTED` | `docs/compliance/fips_alignment.md`; not a CMVP submission |
| CNSA 2.0 alignment | `✅ SELF-TESTED` | PQC cascade verified against CNSA 2.0 primitives |
| External certification | `❌ NOT CLAIMED` | No certification by any standards body is claimed |

---

## Genesis Lock

| Item | Status | Notes |
|------|--------|-------|
| `GENESIS_CONSTANTS.toml` | `⚠️ PROVISIONAL` | `genesis_status = "provisional"`, `deployment_authoritative = false` |
| Genesis hash lock | `❌ BLOCKER` | Requires Outcome A decision (`[genesis-change-acknowledged]` PR) |
| `v1.0-reference` tag | `❌ NOT CREATED` | Reserved for Outcome A; do not create without owner decision |
| `v1.0-rc1` tag | `✅ ACTIVE V1` | RC evidence milestone tag |

---

## Profile Taxonomy

| Profile | Repo | Status | Notes |
|---------|------|--------|-------|
| Pure QASH Core | `corp0ratestarts-cmd/pure-qash` | `📋 POST-V1` | Separate repo; ADR-015; see `docs/spec/19_profile_taxonomy.md` |
| QASH Regulated Profile | `corp0ratestarts-cmd/qash` (umbrella) | `⚠️ INTERFACE-ONLY` | Class IV + disclosure key scaffolding in `crates/pal/src/regulated/`; feature-gated; production HSM key management deferred. ADR-016. |
| QASH Sovereign Hardened Profile | `corp0ratestarts-cmd/qash` (umbrella) | `📋 POST-V1` | Attested HW admission boundary; post-v1 research track |
