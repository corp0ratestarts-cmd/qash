# QASH Security Policy

Internal security policy. Covers key handling, zeroization, dependency policy,
supported claim boundary, vulnerability triage, release evidence, and explicit non-claims.

Version: 1.0-provisional  
Date: 2026-05-30  
Genesis status: provisional (`deployment_authoritative = false`)

---

## 1. Scope

This policy applies to the QASH protocol repository (`corp0ratestarts-cmd/qash`) and all
code within it. It governs:

- Cryptographic key handling and zeroization
- Dependency ingestion and supply-chain security
- Claim boundary (what is and is not asserted)
- Vulnerability triage and disclosure
- Release evidence requirements

---

## 2. Cryptographic Key Handling

### 2.1 Key lifecycle

- Epoch signing keys are generated per-epoch by the PAL (`crates/pal/src/`) and never stored beyond
  their epoch. The blinding key is derived via `derive_epoch_blinding_key()` and lives only in
  stack memory for the duration of a single epoch transition.
- Genesis keys (GRC-7-7-v2 hedge roots) are computed once during genesis certificate generation
  and committed to `GENESIS_CONSTANTS.toml`. They are not secret keys; they are anti-grinding
  commitments derived from public Argon2id outputs.
- No private keys are committed to the repository.

### 2.2 Zeroization

- All temporary key material in Domain B code must be zeroized after use using the `zeroize` crate
  or equivalent. Domain A code (`crates/consensus/`) has `#![forbid(unsafe_code)]` and processes
  no private key material.
- Commitment frames (`CommitmentFrame`) carry only 32-byte public roots; no private key material
  traverses the wire format.

### 2.3 Post-quantum primitives

- Primary signing: Dilithium5 (ML-DSA-87, FIPS 204-aligned)
- Anchor signing: SLH-DSA-SHA3-256 (FIPS 205-aligned)
- Fallback signing: Falcon-512 (scaffold; not activated in v1.0)
- KEM: X-Wing / ML-KEM-768 (FIPS 203-aligned, transport layer only)

These are selected for CNSA 2.0 / post-quantum alignment. **They are not FIPS-validated by a CMVP
lab and are not claimed as such.** See section 6.

---

## 3. Dependency Policy

### 3.1 Supply-chain controls

- All Rust dependencies are pinned via `Cargo.lock` and reviewed for license and security
  via `cargo deny` (`deny.toml`).
- The OSV scanner (`google/osv-scanner-action`) runs on every CI push. Findings not listed in
  `osv-ignore.toml` fail the build. Each exception in `osv-ignore.toml` must include a documented
  rationale.
- No dependency may use floating-point arithmetic in a code path that reaches Domain A state.
- `BTreeMap` is required in Domain A; `HashMap` without a deterministic seed is forbidden.

### 3.2 Updating dependencies

- Major version updates require a changelog entry in the commit.
- Security patches: update and document within 14 days of CVE publication (or justify an exception
  in `osv-ignore.toml`).
- The `cargo deny check` step in CI enforces banned crates, duplicate versions, and license policy.

### 3.3 Fuzzing

The `fuzz/` workspace runs eight fuzz targets via honggfuzz on every PR (10k execs/target) and
weekly at 1M execs/target. Crashes are treated as blocking until triaged and resolved.

---

## 4. Supported Claim Boundary

### 4.1 What is claimed for v1.0 (genesis-provisional)

- **Deterministic consensus**: Every authorized ISA (x86_64, aarch64, riscv64gc) produces
  identical state roots given identical inputs. Verified by multi-compiler differential testing
  (opt-level 0 vs 3, and Cranelift backend).
- **Absorbing halt safety**: If Φ_safety ≥ PHI_MAX_SAFE or any Domain A arithmetic overflows,
  the protocol halts irreversibly. Proved in `proofs/safety/absorbing_halt.v`.
- **Lyapunov stability**: L(t+1) < L(t) iff the admitted transactions are valid. Proved in
  `proofs/contractivity/lyapunov_stability.v`.
- **Cascade collision/preimage resistance**: Rests on AX-3 (SHA3-256 collision resistance,
  assumed). Probabilistic argument in `proofs/cascade/cascade_collision_resistance.v`.
- **Domain A/B partition**: No Domain B value (nondeterminism, wall-clock, unsafe) influences
  a Domain A computation. Enforced by `check_domain_a_tripwires.sh` and `#![forbid(unsafe_code)]`
  in the consensus crate.
- **State-root commitment binding**: The 32-byte state root uniquely identifies epoch state under
  AX-3. Defined by ADR-003 and implemented in `crates/consensus/src/encoding.rs`.

### 4.2 What is not claimed for v1.0

- **Production deployment readiness**: `deployment_authoritative = false` until the genesis lock
  commit. The genesis lock requires manual PDF traceability verification (Phase 1-D) and final
  GRC value regeneration.
- **Formal avalanche proof**: Cascade avalanche is statistical/KAT evidence only, not a genesis
  security proof. See `proofs/privacy/cascade_avalanche_property.v`.
- **ORAM access non-interference**: Excluded from v1.0 active claim boundary.
- **Blinding PRF security (formal game)**: Implemented and tested; formal AU game proof deferred.
- **ZK verification as consensus primitive**: Plonky3 is a Domain B feature; ZK proofs are not
  part of the consensus state-root definition.
- **Real hardware attestation**: TPM/TDX/SEV-SNP/ARM-CCA are scaffolds. Hardware attestation
  correctness requires platform-specific hardware.
- **Threshold signing**: TALUS is a scaffold; not activated.
<!-- claim-boundary-allow: explicit non-claim listing of things we do not assert -->
- **External certification / no FIPS validation / no regulator approval**: See section 6.

---

## 5. Vulnerability Triage

### 5.1 Reporting

Report security issues to: `corp0rate.starts@gmail.com`

Do not open public GitHub issues for security vulnerabilities until a fix is prepared.

### 5.2 Severity tiers

| Tier | Description | Response |
|------|-------------|----------|
| Critical | Domain A safety violation, absorbing halt bypass, state-root forgery | Fix within 48 hours; immediate advisory |
| High | Domain B compromise with potential Domain A influence, key leakage | Fix within 7 days |
| Medium | Supply-chain CVE, CI bypass, evidence-chain integrity issue | Fix within 14 days or justify exception |
| Low | Documentation, advisory static analysis finding | Fix in next scheduled maintenance |

### 5.3 Domain A vs B risk model

A Domain B vulnerability (e.g., a PAL transport bug) that cannot influence Domain A state is
lower severity than a Domain A vulnerability because the state-root commitment remains valid.
The domain partition is a primary risk-reduction control.

---

## 6. Explicit Non-Claims

The following claims are **not made** and must not appear in documentation, marketing, or
operator communication without independent external verification:

| Non-claim | Reason |
|-----------|--------|
| FIPS 140-3 validated | Not submitted to NIST CMVP; no CAVP certificate |
| FIPS 203/204/205 validated | Implementations are FIPS-aligned, not FIPS-validated |
| Externally audited | No paid external audit has been conducted |
| Militarily certified / sovereign certified | Not evaluated by any government certification body |
<!-- claim-boundary-allow: explicit non-claim row in table of prohibited assertions -->
| Payment/settlement/custody deployment readiness | Requires regulatory approval outside this repository |
| CNSA 2.0 compliant (formal) | Aligned by design; not formally assessed |
| Quantum-safe by certification | Post-quantum primitives are selected; formal certification not obtained |
| ZK-proven consensus | ZK is a Domain B feature; not a consensus-layer claim |

---

## 7. Release Evidence Requirements

Before the v1.0 genesis lock (`genesis_status = "locked"`):

1. `verify_genesis_hash.sh` passes: computed hash matches `GENESIS_CONSTANTS.toml`.
2. Manual PDF traceability verification complete: all `docs/traceability.md` citations verified
   against `spec/pdf/QASH_Spec_v1.0.pdf`. See Phase 1-D in the strategic completion plan.
3. GRC-7-7-v2 values regenerated and stable: `work_root` and all 7 hedge roots recorded.
4. All CI blocking jobs pass: determinism, proofs, domain-A-tripwires, supply-chain, vectors.
5. `docs/release/pre_genesis_evidence_snapshot.md` updated with lock commit SHA.
6. Tag `v1.0-reference` placed on the lock commit. This is a reference anchor, not a software release.

---

## 8. Operational Runbook (Provisional)

Full operational runbook is pending production deployment decisions (Phase 4). Provisional guidance:

- **Key rotation**: Not applicable in v1.0 (zero-governance, genesis-locked parameters).
- **Epoch failure response**: If the consensus loop fails to advance within `max_control_loop_latency_ms`
  (450 ms), the PAL Halt implementation fires an absorbing reset. Recovery requires restarting from
  the last committed epoch state root.
- **Snapshot/replay**: Epoch state roots are deterministically replayable from the genesis state
  given the same transaction sequence on any authorized ISA.
- **Patch deployment**: Any change to genesis-locked files invalidates the genesis hash. Such changes
  define a new network and require a new genesis ceremony.
