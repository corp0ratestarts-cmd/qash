# Unlocked Domain A Demo Profile Vectors

**Status:** Test-vector contract for future implementation  
**Depends on:** `docs/mvp/unlocked_domain_a_demo_profile.md`  
**Scope:** Public-export-only replay evidence for `TX-MVP-ReceiptCommit`

## Purpose

This document records the test-vector contract for the unlocked Domain A demo profile. The vectors intentionally use public commitment exports only. They do not include private receipt bodies, raw nonces, workspace salts, disclosure bodies, filesystem paths, or stable user identity fields.

## Public export sequence

The canonical test sequence is generated in `crates/pal/tests/mvp_demo_profile_vectors.rs` using enum-selected deterministic fixture material. The fixture generator is intentionally local to tests and does not use hardcoded byte arrays, string-labeled nonce material, or numeric salt arguments.

The sequence contains two `TxMvpReceiptCommitPublicExport` records:

1. record A: epoch 10
2. record B: epoch 11

Each public export is exactly `TX_MVP_PUBLIC_EXPORT_BYTES` bytes and decodes through `TxMvpReceiptCommitPublicExport::decode`.

## Replay fold

The vector root is computed as:

```text
root_0 = [0; 32]
root_n = SHA3-256("QASH-MVP-DEMO-PROFILE-ROOT\0" || root_{n-1} || len(record_n) || record_n)
```

Where `record_n` is the fixed-size encoding of `TxMvpReceiptCommitPublicExport`.

## Transcript-order rule

Records are replayed in transcript order: the order they appear in the public commitments file (WAL insertion order). The two-record test sequence [A at epoch 10, B at epoch 11] represents A issued before B. Swapping A and B produces a different root — verified by `replay_root_is_order_sensitive_transcript_order_is_the_rule`. Canonical sorting (by epoch, tx_commitment, etc.) is a future aggregation feature and is not applied here.

## Required properties

The vector tests assert:

- valid public exports decode and replay to a stable root;
- wrong version is rejected;
- truncated public export is rejected;
- extra bytes are rejected;
- replay uses public exports only;
- reordering records changes the root (transcript-order is the rule; alternative orderings are distinct transcripts).

## Non-goals

These vectors do not admit `TX-MVP-ReceiptCommit` into locked Domain A. They do not define payment, settlement, custody, identity, hardware attestation, production ZK, or production deployment behavior.

## Future implementation note

A future adapter may reuse the root function or replace it with a versioned profile-specific root. Any replacement must update this document and the tests in the same PR.
