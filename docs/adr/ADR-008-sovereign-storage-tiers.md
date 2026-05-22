# ADR-008: Sovereign Storage Tiers and Shred Commitments

**Status:** Proposed
**Date:** 2026-05-22
**Source:** PR #93 storage architecture review
**PDF authority:** PDF-SILENT

## Context

The latest PR #93 storage comment accepts a hybrid storage model but rejects
using decentralized storage as a single substrate for all QASH data. Raw PII,
decryption keys, signing keys, and sovereign-regulated data have data residency
and erasure requirements that global DHT-style systems cannot satisfy.

## Decision

Schedule a Domain B storage-compliance track based on jurisdiction-bound storage
tiers:

| Tier | Data | Allowed backend | Compliance driver |
|------|------|-----------------|-------------------|
| Tier 0 | state roots, receipt roots, EFB roots | Domain A node database | replay determinism, DORA |
| Tier 1 | PII, KYC, decryption keys | in-country encrypted vault | GDPR, PIPL, KSA PDPL |
| Tier 2 | encrypted receipt blobs | geo-fenced object storage, private IPFS, or regulated Flux use | MiCA, FIPS |
| Tier 3 | public proofs and public transcript archives | global public storage | transparency |

Domain B must expose a `SovereignVault` trait for Tier 1 storage. A storage
profile is genesis-bound configuration: changing the profile defines a new
network or deployment profile, not a runtime governance decision.

Erasure becomes a verifiable Domain B operation:

1. A user submits a signed `ErasureRequest`.
2. Domain B executes `shred_key()` inside the sovereign vault or HSM boundary.
3. Domain B publishes a `ShredCommitment` to the public transcript surface.
4. Auditors verify the commitment exists without learning the shredded key.

## Constraints

- Flux or other global decentralized storage must never store raw PII,
  unencrypted receipts, signing keys, or sovereign-regulated Tier 1 data.
- Tier 1 and Tier 2 encryption keys must be generated, rotated, and destroyed
  inside a FIPS 140-3 L3 HSM/TPM boundary or the deployment's sovereign
  equivalent.
- Cross-border Tier 1 to Tier 2 transfers require an audit log with the legal
  transfer basis.
- Storage rent must be accounted for through blinded fee commitments before any
  indefinite retention claim is made.

## Required Evidence

- `SovereignVault` PAL trait and tests for profile-bound endpoint rejection.
- `ShredCommitment` envelope or public transcript extension with privacy review.
- CI or policy checks that reject PII and unencrypted receipts in Flux uploads.
- HSM/TPM attestation evidence for regulated storage profiles.
- Retention and storage-rent test vectors for receipt archival eligibility.
