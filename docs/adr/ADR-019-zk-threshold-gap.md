# ADR-019: ZK Proof Verification and Threshold Signing — Production Gap

**Status:** Accepted — Post-V1 (ZK-1 through ZK-4, THR-1 through THR-5)  
**Date:** 2026-06-03  
**Authors:** Protocol team  
**Replaces:** None  
**Related:** ADR-009 (Domain B indexing and ZK prover sizing), ADR-013 (v1 backend boundary), `docs/spec/19_profile_taxonomy.md`

---

## Context

Two advanced cryptographic components in Domain B are partially implemented in v1.0:

1. **ZK proof verification** — Plonky3-based proof backend for transaction admission
   (TX-E0 note/nullifier transfers). The interface and shape harness are complete;
   the production circuit is deferred.

2. **Threshold signing (TALUS)** — t-of-n ML-DSA threshold signing where no single
   party holds the full signing key. Type scaffolding is complete; MPC communication
   and real share combination are deferred.

This ADR records the gap, formalises the Domain B proof-admission boundary that will
hold even after production wiring, and defers all ZK and threshold implementation
to post-v1.

---

## Decision: ZK and Threshold Production Gap

### D1 — ZK proof verification (Domain B boundary, normative)

The Domain B proof-admission boundary is normative and does not change between
the current interface-only state and the production implementation:

```
Domain B (PAL) receives:  raw proof bytes (nullifiers, commitments, Plonky3 proof)
Domain B verifies:        Plonky3 proof using the backend in crates/pal/src/zk/
Domain B produces:        CapToken<ValidatedEffect> containing commitment roots only
Domain A receives:        CapToken<ValidatedEffect> — never raw proof bytes or witness data
```

Raw proof bytes and intermediate witness data MUST NOT enter Domain A. Domain A
performs no ZK verification. This invariant is enforced by the type system and
is not affected by the production gap.

### D2 — v1.0 ZK scope (interface-only)

| Component | File | Status |
|-----------|------|--------|
| `PlonkyProofBundle` type (proof bytes + public inputs) | `zk/backend.rs` | `✅ ACTIVE V1` |
| `ZkVerifier` trait | `zk/backend.rs` | `✅ ACTIVE V1` |
| `ZkProfile` (circuit parameter caps) | `zk/profile.rs` | `✅ ACTIVE V1` |
| Fibonacci AIR toy circuit (shape harness only) | `zk/fib_air.rs` | `⚠️ INTERFACE-ONLY` |
| Plonky3 backend wiring | `zk/plonky3.rs` | `⚠️ INTERFACE-ONLY` |
| QASH transaction ZK circuit (TX-E0) | Not yet implemented | `📋 POST-V1` |

### D3 — ZK production gap (ZK-1 through ZK-4)

| Task | Description | Target |
|------|-------------|--------|
| ZK-1 | This ADR | ✅ Done |
| ZK-2 | Implement QASH TX-E0 note/nullifier circuit using Plonky3 | Post-v1 |
| ZK-3 | Wire Plonky3 backend to `ZkVerifier` trait; replace shape harness | Post-v1 |
| ZK-4 | `CapToken<ValidatedEffect>` production wiring in PAL admission path | Post-v1 |

### D4 — Threshold signing TALUS (demo-only)

The TALUS threshold signing implementation in `crates/pal/src/threshold/talus.rs`
provides the type scaffold (`SignatureShare`, `CombinedSignature`, `ThresholdError`)
behind the `threshold-signing` feature flag.

The current `combine_shares()` implementation uses an XOR placeholder (bitwise XOR
of share bytes) as a stand-in for real t-of-n ML-DSA combination. This produces
structurally valid output but is cryptographically meaningless. It is gated behind
`--features threshold-signing` and marked `⚠️ DEMO-ONLY` in the implementation matrix.

**The XOR placeholder MUST NOT be used in production.** Any deployment that enables
`--features threshold-signing` must confirm it has replaced `combine_shares()` with
the real MPC implementation (THR-3).

### D5 — Threshold production gap (THR-1 through THR-5)

| Task | Description | Target |
|------|-------------|--------|
| THR-1 | This ADR | ✅ Done |
| THR-2 | Define secure channel protocol between TALUS signers | Post-v1 |
| THR-3 | Replace `combine_shares()` XOR placeholder with real ML-DSA share combination | Post-v1 |
| THR-4 | Implement signer enrollment and key generation ceremony | Post-v1 |
| THR-5 | Integration test suite: t-of-n correctness, insufficient shares rejection, timeout | Post-v1 |

### D6 — PQC crypto-agility (interface-only)

The PQC crypto-agility driver (`crates/pal/src/crypto/`) implements the suite-gate
logic (selecting Dilithium5 / SLH-DSA / Falcon-512 based on epoch and agility epoch
constants). The signing drivers themselves are interface-only — they return structurally
valid signatures via the hosted PAL but without real PQC hardware acceleration.

PQC migration at `pqc_agility_epoch = 10000` is defined in genesis constants but
not yet reached. The agility driver is correct; production PQC signing acceleration
is a platform integration concern (post-v1).

---

## Consequences

**Positive:**
- The Domain B proof-admission boundary (`CapToken<ValidatedEffect>`) is enforced
  by the type system today and will continue to hold when ZK-2..4 land.
- TALUS `SignatureShare` and `CombinedSignature` types are stable — THR-3 only
  needs to replace the `combine_shares()` body, not the types.
- Plonky3 dependency is already wired in the workspace; ZK-3 is a backend
  implementation, not a new dependency.

**Negative:**
- Production ZK-based admission (TX-E0 note/nullifier transfers) is unavailable in v1.0.
- Threshold signing is demo-only; single-party signing is used in v1.0.

**Safety note:**
- The implementation matrix and `docs/release/pre_genesis_evidence_snapshot.md`
  explicitly classify `combine_shares()` as demo-only. Deployment operators MUST NOT
  enable `--features threshold-signing` in production until THR-3 is complete.

**Deferred:**
- All of ZK-2 through ZK-4 and THR-2 through THR-5 (see tables in D3 and D5).
- PQC hardware acceleration integration.
