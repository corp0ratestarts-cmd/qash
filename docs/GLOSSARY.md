# QASH Protocol Glossary

Alphabetically-sorted definitions for terms used throughout the codebase,
specifications, proofs, and architecture decision records.

## A

**Absorbing halt** — An irreversible protocol halt triggered when a Domain A invariant
is violated (overflow, Φ_safety threshold breach, etc.). Once halted, the state machine
accepts no further transitions. Implemented via `Halt::absorbing_reset()` in the PAL.
See: `docs/spec/05_absorbing_halt.md`, ADR-004.

**ADR** — Architecture Decision Record. Documents an engineering decision that fills a
PDF-silent gap, chooses between implementation strategies, or defines a layer boundary.
See: `docs/adr/README.md`.

**AX-2** — Compiler correctness axiom. States that the Coq-extracted OCaml model and
the Rust implementation are observationally equivalent. Supported by CI vector witnesses.
See: `proofs/model/RefinementStatement.v`.

**AX-3** — Cryptographic assumption axiom. States that SHA3-256 is collision-resistant.
The v1.0 state-root commitment security rests on AX-3.
See: `proofs/cascade/cascade_collision_resistance.v`.

## B

**Blinding key** — An epoch-scoped key derived via `derive_epoch_blinding_key()` using
`H_cascade_keyed`. Used to mask validator indices in commitment transcripts. Domain B only.

**BTreeMap** — Required map type for deterministic iteration order in Domain A code.
`HashMap` is forbidden in `crates/consensus/` because iteration order is platform-dependent.

## C

**Capability token** — A type-erased wrapper (`CapToken<T>`) that makes cross-domain
value flow explicit. `into_inner()` is the sole observation path. See: `crates/consensus/src/domain.rs`.

**Cascade** — The QASH-CASCADE-7 multi-hash construction. Combines seven L1 512-bit
primitives in parallel, binds them via SHA3-512, and folds the result through expansion
and finalization layers. See: `crates/consensus/src/cascade.rs`, `docs/spec/04_cascade.md`.

**CommitmentFrame** — The fixed-size (144-byte) wire format for zero-persistence
commitment transport. Magic header `QPCOMM1\0` + epoch + four 32-byte roots.
See: `crates/pal/src/commitment_transport.rs`.

**Compatibility window** — The epoch threshold (default: 100) after which v1.0 envelopes
are rejected. v1.1+ envelopes are always accepted. See: ADR-012, `[cascade] compatibility_window`.

## D

**Deployment authoritative** — `GENESIS_CONSTANTS.toml` field. `false` while genesis is
provisional; `true` only after the genesis lock commit.

**Domain A** — The deterministic consensus domain. All code in `crates/consensus/`. No
`unsafe`, no `f32`/`f64`, no `HashMap`, no wall-clock, all arithmetic checked.

**Domain B** — The PAL / operational domain. Code in `crates/pal/` and `src/`. May use
`unsafe` under audit. Domain B values must never flow into Domain A computations.

## E

**ACVP KAT** — Known-Answer Test vector in the NIST Automated Cryptographic Validation Protocol JSON format. QASH provides ACVP-style fixtures in `tests/cavp/` as internal evidence; no NIST ACVP submission has been made. See: `docs/compliance/kat_policy.md`.

**EFB** — Epoch Finality Beacon. A 32-byte root committing to cross-shard receipts for
an epoch. Included in the `CommitmentFrame`. See: `crates/consensus/src/sharding.rs`.

**Encoding** — The canonical byte layout of `EpochState` for state-root commitment.
Defined by ADR-003 and `ADR-012`. Implemented in `crates/consensus/src/encoding.rs`.

**`EncodeError::ValueOutOfRange`** — Error variant returned by `try_encode_full_state_into`
when a FixedPoint field does not fit in i64 without saturation. Callers needing to detect
overflow before encoding should use `try_encode_full_state_into` instead of
`encode_full_state_into`. See: `crates/consensus/src/encoding.rs`.

**Envelope** — A fixed-capacity input buffer `Envelope<N>` carrying a transaction and
its post-quantum signature into the consensus pipeline. See: `crates/consensus/src/envelope.rs`.

**Epoch** — The fundamental time unit of the QASH protocol (500 ms). Each epoch runs
one transition: validate → admit transactions → evaluate Lyapunov → commit.

**`ErasureRequest`** — A structured intake record for an Art. 17 GDPR erasure request,
carrying the `receipt_commitment` (SHA3-256 of the key), `requestor_id`, and `epoch`.
Consumed by `process_erasure_request`. See: `crates/pal/src/privacy/erasure.rs`,
`docs/security/ERASURE_RUNBOOK.md`.

## F

**FIPS POST (Power-On Self-Test)** — A startup self-test running Known-Answer Tests for all
in-boundary Domain B cryptographic algorithms (SHA3-256, SHA-256, HMAC-DRBG, ML-KEM-768).
Enabled by the `fips-post` feature. See: `crates/pal/src/crypto/post.rs`, `docs/compliance/fips_compliance.md`.

**FixedPoint** — A scaled integer type `FixedPoint { raw: i128 }` with scale factor
`1_000_000`. All Lyapunov arithmetic uses fixed-point to avoid floating-point non-determinism.
See: `crates/consensus/src/fixed_point.rs`.

**Fingerprint** — A causal chain fingerprint `fp = H(fp_prev, epoch, root)` that makes
history forgery detectable. See: `crates/consensus/src/transition.rs`.

## G

**Genesis constants** — The immutable set of protocol parameters in `GENESIS_CONSTANTS.toml`.
Changing them defines a new network. Treat as append-only.

**Genesis hash** — `QASH-CASCADE-7:` + hex digest over the canonical artifact set defined
in `spec/genesis-artifacts.txt`. Computed by `cargo run --bin genesis-hash`.

**Genesis lock** — The final commit setting `genesis_status = "locked"` and
`deployment_authoritative = true`. Tagged `v1.0-reference`. Not a general software release.

**GRC-7-7-v2** — Genesis Reference Certificate, version 7-7-v2. An Argon2id-based
anti-grinding and anti-precomputation certificate. 7-of-7 hedge roots.
See: `src/bin/genesis_cert.rs`.

## H

**H_cascade** — `h_cascade(input: &[u8]) -> [u8; 64]`. The public (unkeyed) cascade hash.
Domain A pure function. See: `crates/consensus/src/cascade.rs::h_cascade`.

**H_cascade_keyed** — `h_cascade_keyed(context_key: &[u8], input: &[u8]) -> [u8; 64]`.
The keyed variant used for blinding key derivation. Domain B only.

**H_domain** — Domain-separated hash: `SHA3-256(tag_u32_le || input)`. Used for v1.0
state-root commitments. See: ADR-003, `crates/consensus/src/transition.rs::compute_state_root`.

**Halt** — See *Absorbing halt*.

## I

**IT-MAC** — Information-theoretic MAC over GF(2¹²⁸) used in the cascade derive path.
Forgery bound: 16/2¹²⁸. Domain B / Phase 2 feature; not an active v1.0 claim.

## L

**Lineage** — A skip-list-compressed chain of epoch headers used for fast replay verification.
See: `crates/consensus/src/lineage.rs`.

**Lyapunov potential** — `L(t) = V_convergence(t) + Φ_safety(t)`. If `L(t+1) ≥ L(t) + ε`
the step is rejected. See: `crates/consensus/src/lyapunov.rs`, `proofs/contractivity/`.

## M

**MAX_VALIDATORS** — `1024`. Maximum number of validators per epoch. Array-bounded, no heap.

## P

**PAL** — Platform Abstraction Layer. `crates/pal/`. Defines `Time`, `Net`, `Attest`, `Halt`
traits and their hosted implementations. Domain B.

**Phi safety** — `Φ_safety(t) = W_S · Σ_i slash_accumulator_i(t)`. If `Φ_safety ≥ PHI_MAX_SAFE`
the H7 absorbing halt triggers. See: ADR-001, ADR-002, `lyapunov.rs`.

**Post-quantum** — Refers to the cryptographic algorithms resistant to quantum computer
attacks: Dilithium5 (primary), SLH-DSA-SHA3-256 (anchor), Falcon-512 (fallback).

**Provisional** — `genesis_status = "provisional"`. The genesis hash is computed but not
locked. `deployment_authoritative = false`. See: `GENESIS_CONSTANTS.toml`.

## Q

**QASH-CASCADE-7** — The lock algorithm identifier. References the 7-primitive cascade
hash construction used for the genesis hash and commitment binding.

## R

**Replay invariant** — A consensus transition that produces the same output on any
authorized ISA (x86_64, aarch64, riscv64gc) given the same input. All Domain A code
must be replay-invariant.

## S

**State root** — `compute_state_root(state, crypto_suite)`. The canonical 32-byte
commitment to an epoch state. v1.0: `H_domain(STATE_ROOT, Encode_for_commitment(state))`.
See: ADR-003, ADR-012.

**Supply chain** — Dependency provenance and security. Governed by `deny.toml` (cargo-deny),
`osv-ignore.toml` (OSV scanner exceptions), and the SBOM artifact.

## T

**TOE (Target of Evaluation)** — In Common Criteria, the system under formal evaluation.
QASH's TOE boundary is Domain A (`crates/consensus/`) plus selected Domain B crypto
(`crates/pal/src/crypto/kem.rs`, `drbg.rs`, `privacy/erasure.rs`). See: `docs/compliance/cc_security_target.md`.

**Traceability** — `docs/traceability.md`. Maps each protocol property (P0-1 through P0-9)
to its PDF citation, ADR, Coq theorem, Rust implementation, and CI test.

**`try_encode_full_state_into`** — Fallible state encoder. Returns `Err(EncodeError::ValueOutOfRange)`
if any FixedPoint validator or window field does not fit in i64, rather than saturating.
See: `crates/consensus/src/transition.rs`.

**TH-n** — Theorem n. References a named theorem in `proofs/COVERAGE.md`.

**TX-0** — A no-op/initialization transaction type. Admitted unconditionally in v1.0.

**TX-1** — A validator divergence update transaction. Signed with a post-quantum key.

## V

**v1.0-reference** — The genesis lock tag. Not a general software release; a reference
anchor for the locked genesis state. Semantically equivalent to a git tag on the lock commit.

**ValidatorMetrics** — The per-validator state `{ divergence, conflict, slash_accumulator }`.
See: `crates/consensus/src/transition.rs`.

## W

**Wire format** — The canonical byte encoding of protocol messages (transactions, envelopes,
commitment frames) used for network transmission and deterministic hashing.
