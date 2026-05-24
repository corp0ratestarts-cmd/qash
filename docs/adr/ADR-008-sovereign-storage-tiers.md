# ADR-008: Sovereign Storage Tiers

**Status:** Proposed  
**Date:** 2026-05-24  
**Source:** PR #93 follow-through reconciliation

## Context

QASH needs jurisdiction-neutral protocol semantics while allowing regulated
operators to satisfy local storage and erasure obligations. Decentralized storage
cannot be treated as a universal substrate for all data classes.

## Decision

Define storage tiers as Domain B deployment policy. Domain A stores and commits
only roots and scalar public commitments.

Tiers:

- **Tier 0:** Domain A state roots, receipt roots, EFB roots, and public transcript roots.
- **Tier 1:** PII, KYC, decryption keys, and local regulated secrets in in-country encrypted vaults.
- **Tier 2:** encrypted receipt blobs in geo-fenced object storage, private IPFS, regulated Flux use, or equivalent storage.
- **Tier 3:** public proofs, public transcript archives, and non-sensitive evidence bundles.

Flux/IPFS-like storage is never allowed for raw PII, unencrypted receipt bodies,
signing keys, or Tier 1 regulated material.

## Required implementation

- Add a `SovereignVault` PAL trait for Tier 1 storage.
- Add profile-bound endpoint rejection tests.
- Add `ErasureRequest` and `ShredCommitment` as auditable Domain B events.
- Bind regulated storage keys to FIPS 140-3, PKCS#11, TPM, or sovereign-equivalent local custody profiles where required by deployment policy.
- Keep storage-rent and retention policy in Domain B economics; do not add minting, validator rewards, or governance-controlled subsidies.

## Consequences

This permits regulated deployments without polluting Domain A with jurisdiction,
operator, hardware, or storage-provider state.
