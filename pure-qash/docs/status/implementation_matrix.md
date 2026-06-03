# Pure QASH Implementation Status Matrix

**Date:** 2026-06-03  
**Milestone:** P-0 scaffold (pre-RC)  
**Genesis status:** provisional  

---

## Status taxonomy

| Label | Meaning |
|-------|-------|
| `✅ DONE` | Implemented, tested, in CI |
| `🔧 IN PROGRESS` | Work started, not yet merged |
| `📋 TARGET` | Planned, not yet started |
| `❌ EXCLUDED` | Explicitly absent by design |

---

## P-series progress

| PR | Task | Status | Notes |
|----|------|--------|-------|
| P-0 | Repo scaffold, CLAUDE.md, CI skeleton | `✅ DONE` | This scaffold branch |
| P-1 | Pure QASH claim boundary | `✅ DONE` | `docs/claims/pure_qash_claims.md` |
| P-2 | Pure privacy model, no Class IV | `✅ DONE` | `docs/spec/09_privacy_model.md` |
| P-3 | Constitutional Scarcity Axiom spec | `✅ DONE` | `docs/spec/08_tokenomics.md` |
| P-4 | Pure GENESIS_CONSTANTS.toml + schema | `✅ DONE` | Provisional constants |
| P-5 | EconomicsState + EpochState integration | `🔧 IN PROGRESS` | Module written; EpochState integration requires full consensus import |
| P-6 | Domain A economics functions | `✅ DONE` | `crates/consensus/src/economics.rs` |
| P-7 | PTX-0 / Pure cash transfer spec | `✅ DONE` | `docs/spec/16_pure_qash_transfer.md` |
| P-8 | OrderImage per-type rule | `✅ DONE` | Defined in `docs/spec/08_tokenomics.md §T4` |
| P-9 | MEV-null transaction law | `✅ DONE` | Defined in `docs/spec/08_tokenomics.md §T3` |
| P-10 | zero-persistence pure-qash feature | `✅ DONE` | `crates/pal/src/admission/ephemeral.rs`, `wal/production.rs` |
| P-11 | blind certification evidence boundary | `✅ DONE` | `docs/spec/17_blind_certification_evidence.md` |
| P-12 | theorem target scaffolding | `✅ DONE` | All 19 theorems as TARGET stubs |
| P-13 | absence guards | `✅ DONE` | `scripts/check_pure_absence_guards.sh` |
| P-14 | extend xtask | `✅ DONE` | `xtask/src/main.rs` — 7 commands |
| P-15 | full CI suite | `✅ DONE` | `.github/workflows/ci.yml` (11 jobs) |
| P-16 | evidence snapshot | `✅ DONE` | Template at `docs/release/pure_qash_rc_evidence_snapshot.md` |
| P-17 | RC milestone tag | `📋 TARGET` | Requires all CI green on populated evidence |
| P-18 | genesis-candidate (owner-gated) | `📋 TARGET` | Future; requires `[pure-qash-genesis-candidate-acknowledged]` |

---

## Explicitly excluded (by design — absence guards enforce)

| Concept | Why excluded |
|---------|-------------|
| Class IV observer | Regulated Profile only; not Pure QASH |
| Disclosure key | No lawful-basis disclosure in Pure QASH |
| Priority fees | MEV-null by construction |
| Fee auction / mempool ordering | MEV-null by construction |
| Validator fee revenue | All fees burn |
| Monetary governance | Genesis-locked constants |
| Oracle supply inputs | Constitutional scarcity only |
| Raw TX retention (production) | Zero-persistence production profile |
| Peer IP in durable store | Zero-persistence production profile |
