# QASH Formal Hazard Analysis (HAZOP)

Component × hazard scenario matrix for the QASH protocol.

Version: 1.0-provisional  
Date: 2026-05-30  
Method: HAZOP (Hazard and Operability Study) adapted for distributed consensus systems.

---

## Scope

Components analyzed:

1. Consensus engine (Domain A — `crates/consensus/`)
2. Platform Abstraction Layer (Domain B — `crates/pal/`)
3. Clone protocol (multi-channel offline transport)
4. Epoch blinding / commitment transport
5. Hardware attestation (scaffolded)

---

## HAZOP Table

### Component 1: Consensus Engine (Domain A)

| Scenario | Hazard | Cause | Effect | Safeguard / Mitigation | Accepted axiom | Residual risk |
|----------|--------|-------|--------|------------------------|----------------|---------------|
| Arithmetic overflow in Lyapunov evaluation | State corruption / non-determinism | Adversarial transaction sequence with large divergence values | Incorrect L(t) comparison; protocol state diverges across replicas | `checked_add/mul` everywhere; overflow triggers `Halt::absorbing_reset()`. Proved in `proofs/safety/absorbing_halt.v`. | None | Low: absorbing halt is irreversible; recovery requires replay from genesis |
| `f32`/`f64` in Domain A path | Non-determinism across ISAs | Developer error; dependency introducing float | State roots differ between x86_64 and riscv64gc; consensus failure | `#![forbid(unsafe_code)]`; `check_domain_a_tripwires.sh` scans for float; multi-compiler CI diff | None | Low: CI gate is blocking |
| `HashMap` non-determinism | State root varies by seed | Developer uses `HashMap` instead of `BTreeMap` in state struct | Two replicas with same inputs produce different roots | `check_domain_a_tripwires.sh`; clippy | None | Low: CI gate is blocking |
| `usize`/`isize` in wire-format arithmetic | Platform-dependent state | 32-bit vs 64-bit ISA difference in struct field size | State root differs across platforms | Forbidden in state struct fields; `check_domain_a_tripwires.sh` awk struct-body scanner | None | Low: CI gate catches new violations |
| Φ_safety threshold breach | Validator set compromise | Coordinated slash accumulation by ≥ f validators | Absorbing halt triggered (protocol terminates) | Lyapunov halt gate; PHI_MAX_SAFE = 500_000_000 in `GENESIS_CONSTANTS.toml` | None | Medium: absorbing halt is the intended response; recovery requires restart |
| Epoch counter wrap | History forgery | u64 epoch counter exhaustion (18×10¹⁸ epochs ≈ 285 billion years) | Fingerprint collision risk | u64 epoch counter; effectively unbounded | None | Negligible |
| Stale cross-shard receipt replay | Invalid EFB state root | Attacker replays EFB root from an earlier epoch | Invalid state accepted | Causal fingerprint chain `fp = H(fp_prev, epoch, root)` in `transition.rs` | None | Low |

### Component 2: Platform Abstraction Layer (Domain B)

| Scenario | Hazard | Cause | Effect | Safeguard / Mitigation | Accepted axiom | Residual risk |
|----------|--------|-------|--------|------------------------|----------------|---------------|
| Domain B value leaks into Domain A | Protocol violation | Developer passes PAL timestamp or RNG output into consensus state | Non-determinism in state root | Domain A/B partition; `check_domain_a_tripwires.sh`; `check_zero_persistence_boundary.sh`; code review | None | Medium: static analysis catches obvious cases; subtle data flows require manual review |
| Unsafe code introduces memory unsafety | Crash / data exposure | PAL `unsafe` block with UAF or OOB write | Validator crash; potential key material exposure | `cargo miri test` on consensus (Domain A); PAL unsafe under audit policy | None | Medium: PAL unsafe is audited but not formally verified |
| Blinding key reuse across epochs | Privacy linkability | PAL fails to rotate blinding key at epoch boundary | Validator indices linkable across epochs | `derive_epoch_blinding_key()` is epoch-scoped; epoch counter is mandatory input | Blinding PRF security (deferred; see `proofs/blinding/blinding_non_interference.v`) | Medium: formal PRF game proof deferred post-genesis |
| CommitmentFrame magic header spoofing | Replayed or forged commitment | Attacker crafts a frame with valid magic but stale roots | Stale state accepted as current | Epoch field in frame; fingerprint chain | None | Low: epoch field makes replays detectable |
| CommitmentFrame buffer overflow | Crash / code execution | Malformed frame with oversized payload | PAL crash | Fixed 144-byte frame size; `COMMITMENT_FRAME_BYTES` const; `boundary_fuzz` target | None | Low: fuzz target covers encode/decode roundtrip |

### Component 3: Clone Protocol (Offline Transport)

| Scenario | Hazard | Cause | Effect | Safeguard / Mitigation | Accepted axiom | Residual risk |
|----------|--------|-------|--------|------------------------|----------------|---------------|
| Chunk replay / deduplication failure | Duplicate transactions admitted | Bloom filter false negative; relay node resends | Epoch state double-counts a validator update | Bloom filter deduplication + Dilithium5 signature nonce | None | Low: double-admission requires bypassing both controls |
| Cover traffic timing attack | Validator identity leakage | Attacker correlates packet timing with validator activity | Epoch-level validator identity linkage | Constant-rate dummy emissions (`cover_traffic = true`) | Blinding PRF security (deferred) | Medium: statistical timing attacks may remain; formal proof deferred |
| Emergency wipe signal forgery | Validator state wiped by attacker | Attacker forges a wipe signal with a compromised key | Validator loses state; must resync from genesis | Wipe signal requires valid Dilithium5 signature from the validator's own key | Dilithium5 EUF-CMA (AX-3 dependent) | Low: requires full key compromise |
| QR/NFC chunk injection | Malicious chunk admitted | Attacker injects a crafted chunk via QR code or NFC tap | Invalid transaction admitted to consensus | Envelope signature verification; Dilithium5 primary key | Dilithium5 EUF-CMA | Low |
| Max offline epoch exhaustion | State divergence | Validator offline > `max_offline_epochs` (12) | Validator falls behind; cannot catch up without full replay | Replay invariant; state root replay from any authorized ISA | None | Medium: validator must have access to epoch history |

### Component 4: Epoch Blinding / Commitment Transport

| Scenario | Hazard | Cause | Effect | Safeguard / Mitigation | Accepted axiom | Residual risk |
|----------|--------|-------|--------|------------------------|----------------|---------------|
| Blinding key derivation collision | Two validators share blinding key | H_cascade_keyed collision | Validator index linkage | AX-3 (SHA3-256 collision resistance); 512-bit output space | AX-3 | Negligible under AX-3 |
| Blinding key oracle attack | Attacker recovers blinding key from transcript | Transcript leaks enough for key reconstruction | Privacy violation | Zero-persistence: commitment frames carry no key material | Blinding PRF security (deferred) | Medium: formal PRF proof deferred |
| State root forgery | Attacker presents a false state root | SHA3-256 second-preimage attack | Invalid state accepted as valid | AX-3; compute_state_root in `transition.rs` | AX-3 | Negligible under AX-3 |

### Component 5: Hardware Attestation (Scaffolded)

| Scenario | Hazard | Cause | Effect | Safeguard / Mitigation | Accepted axiom | Residual risk |
|----------|--------|-------|--------|------------------------|----------------|---------------|
| TEE attestation bypass | Validator claims hardware it does not have | Software stub returns success without real TEE | Unattested validator participates as if attested | `require_hardware_tee = false` in v1.0; attestation is advisory | None | High: real TEE verification is a scaffold; this is an accepted v1.0 limitation |
| TPM 2.0 key seal failure | Validator key material unsealed from wrong PCR | PCR state manipulated before seal | Validator key exposed to wrong software state | PAL attestation gate; platform policy | None | High: requires real TPM hardware; scaffold only in v1.0 |

---

## Summary: Residual Risk Register

| Risk | Severity | Accepted at | Mitigation path |
|------|----------|-------------|-----------------|
| Absorbing halt is irreversible (intended behavior) | Low (by design) | v1.0 genesis | Recovery via genesis replay |
| Domain B unsafe code (audited, not formally verified) | Medium | v1.0 genesis | Post-genesis: formal PAL audit; Miri CI |
| Blinding PRF security (no formal AU game proof) | Medium | v1.0 genesis | Phase 3-A: formal game proof post-genesis |
| Cover traffic residual timing | Medium | v1.0 genesis | Phase 3-A/3-D: formal privacy proof post-genesis |
| Hardware attestation is scaffold | High (known gap) | v1.0 genesis (non-claim) | Phase 4-A: real TPM/TDX/SEV-SNP post-genesis |
| ZK verification is Domain B only | High (known gap) | v1.0 genesis (non-claim) | Phase 4-D: Plonky3 real verification post-genesis |
| No FIPS validation | N/A (non-claim) | Policy | External process; not a repo gate |

---

## Axioms Relied Upon

| Axiom | Reference | Status |
|-------|-----------|--------|
| AX-3 (SHA3-256 collision resistance) | `proofs/cascade/cascade_collision_resistance.v` | Accepted cryptographic assumption |
| Dilithium5 EUF-CMA | `GENESIS_CONSTANTS.toml` | Accepted — NIST FIPS 204 post-quantum assumption |
| ML-KEM-768 IND-CCA2 | `GENESIS_CONSTANTS.toml` | Accepted — NIST FIPS 203 post-quantum assumption |
| GRC-7-7-v2 anti-grinding | `src/bin/genesis_cert.rs` | Accepted computational assumption; Argon2id cost parameters |
