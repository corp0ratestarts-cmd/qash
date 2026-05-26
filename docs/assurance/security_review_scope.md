# Security Review Scope — QASH Pilot Baseline

**Version:** Pilot Baseline v0.2  
**Review type:** Pre-pilot security scoping document

---

## What Is In Scope

### Domain A — Deterministic Consensus Core (`crates/consensus`)

- Fixed-point arithmetic overflow → absorbing halt behaviour
- Nonce replay prevention in `prevalidate_all`
- Lyapunov stability window and convergence halt logic
- State root commitment integrity (SHA3-256 domain-separated fold)
- Canonical transaction ordering (sort key determinism)
- Encoding/decoding round-trip correctness (`encode_full_state_into` / `decode_full_state`)

### Domain B — Platform Abstraction Layer (`crates/pal`)

- Write-ahead log append-only invariant
- Privacy boundary: no incident body in public commitment export
- Replay determinism across two consecutive runs
- Selective disclosure isolation (only the addressed record is decrypted)
- Recovery WAL replay correctness

### Supply Chain

- Dependency audit (OSV, cargo-deny)
- SBOM generation and freshness
- Pinned Rust toolchain (`rust-toolchain.toml`)

---

## What Is Out of Scope

- Production post-quantum signature verification (signatures are carried opaquely in Domain A at this TRL)
- Production hardware attestation (TPM/TEE integration is a future phase)
- Network protocol security (no network stack at this TRL)
- Key management infrastructure
- Multi-operator transcript merging (v0.3 scope)
- Regulatory compliance certification (NIS2, DORA, GDPR)

---

## Existing Assurance Evidence

| Evidence | Location | Status |
|---|---|---|
| Coq proof: Lyapunov stability | `proofs/contractivity/lyapunov_stability.v` | Complete |
| Coq proof: Absorbing halt safety | `proofs/safety/absorbing_halt.v` | Complete |
| CI: test-determinism (cross-ISA) | `.github/workflows/platform-determinism.yml` | Runs on every PR |
| CI: zero-persistence boundary | `.github/workflows/ci.yml` | Runs on every PR |
| CI: domain boundary tripwires | `.github/workflows/genesis-guard.yml` | Runs on every PR |
| CI: fuzz-smoke | `.github/workflows/fuzz-smoke.yml` | Runs on every PR |
| CI: supply-chain (OSV + cargo-deny) | `.github/workflows/ci.yml` | Runs on every PR |
| Post-merge audit | `docs/mvp/post_merge_audit.md` | Manual, per-release |

---

## Known Limitations

1. **No production signature verification.** The `PQ_SIG_BYTES` field is carried as opaque bytes; Domain A does not verify signatures. A production deployment requires a Domain B PAL implementation of Dilithium5 + SLH-DSA-SHA3-256 verification.

2. **Single-operator only.** The pilot baseline supports one operator workspace. Multi-operator transcript merging is deferred to v0.3.

3. **Synthetic data only.** No threat modelling has been done for production incident data ingestion pipelines.

4. **No key rotation.** Validator IDs are fixed per genesis; key rotation is not implemented.

---

## Recommended Pre-Production Review Steps

1. Commission an independent cryptographic review of the Domain A hash cascade and domain separation tags.
2. Commission an independent review of the PAL hosted module (Domain B) for privacy boundary correctness.
3. Complete TH-9/TH-10/TH-11 Coq proofs (`proofs/cascade/`).
4. Integrate production-grade Dilithium5 signature verification in Domain B.
5. Conduct a structured threat model review against the QASH threat model (`docs/threat_model/`).
