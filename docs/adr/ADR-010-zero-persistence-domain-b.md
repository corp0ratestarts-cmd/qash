# ADR-010: Zero-Persistence Domain B Admission

**Status:** Proposed  
**Date:** 2026-05-23  
**Source:** Domain B black-hole / zero-persistence review  
**PDF authority:** PDF-SILENT

## Context

QASH already separates deterministic Domain A consensus from nondeterministic
Domain B platform behavior. Recent privacy, storage, and sharding work requires a
more precise production rule for raw transaction-graph material.

A policy that says "do not publish the graph" is insufficient. A production
Domain B implementation must ensure that raw graph material has no admissible
software path to become durable, serializable, logged, replayed, or publicly
emitted.

The current hosted PAL replay scaffold is intentionally useful for pre-genesis
vectors and evidence capture, but it is not the production privacy posture. A
separate production profile is required.

## Decision

Adopt **Zero-Persistence Graph Non-Publication** as the Domain B production
privacy invariant.

The implementation will be split into explicit profiles:

- `replay-scaffold`: evidence and golden-vector mode; raw fixtures may be
  persisted only for test/replay artifacts.
- `zero-persistence`: production admission mode; raw graph material is confined
  to owned ephemeral admission slots and cannot be written to WAL, logs, metrics,
  audit events, or public transcript surfaces.
- `sovereign-hardened`: zero-persistence plus hardware isolation and attestation,
  including DPU/SmartNIC admission, confidential host memory, IOMMU lockdown, and
  HSM/TPM-bound key erasure evidence.

Domain A remains unchanged. It receives only validated scalar effects, public
roots, and other fixed-width values admitted by the relevant consensus spec.

## Required Software Properties

The zero-persistence profile requires:

1. Owned `EphemeralEnvelope` admission types that are non-serializable,
   non-debuggable, non-cloneable, and consumed by value.
2. In-place parsing through borrowed views, not owned payload copies.
3. `CapToken<ValidatedEffect>` or equivalent schema-validated scalar crossing at
   the Domain A boundary.
4. Production WAL schema that has no `raw_txs`, raw envelope, peer metadata,
   receipt body, or graph-topology fields.
5. Blind audit events that record security boundary events without payloads,
   peer addresses, receipt bodies, or graph topology.
6. CI gates and model-checking harnesses that prevent accidental regression.

## Sovereign Hardened Requirements

The Sovereign Hardened profile additionally requires:

- raw packet admission in an attested DPU/SmartNIC or equivalent isolated
  boundary before host memory,
- host receipt of scalar commitments or bounded admission records only,
- IOMMU policy that restricts DMA to approved memory windows,
- confidential-memory support such as SEV-SNP, TDX, or CCA where available,
- attested firmware and admission-code measurement,
- HSM/TPM or sovereign-equivalent key generation and erasure evidence for Tier 1
  and Tier 2 storage profiles.

These are profile requirements, not baseline claims about every QASH deployment.

## Consequences

### Positive

- Gives graph non-publication a concrete implementation lifecycle.
- Resolves the hosted replay scaffold versus production privacy tension.
- Provides a clean place for DPU, TDX/SNP/CCA, IOMMU, and Kani evidence without
  contaminating Domain A.
- Strengthens compliance language around erasure, blind audit, and public
  transcript minimization.

### Negative / Cost

- Requires feature-gated PAL architecture and compile-fail tests.
- Requires additional evidence before production privacy claims are allowed.
- Sovereign Hardened deployments require specialized hardware, attestation
  operations, and operational discipline.
- Absolute legal/business claims remain out of scope for the engineering spec.

## Required Evidence Before Implementation Complete

- `docs/spec/14_zero_persistence_pipeline.md` merged and linked from roadmap and
  release checklists.
- `cargo check -p qash-pal --no-default-features --features zero-persistence`.
- Tests proving production WAL rejects raw graph material.
- Tests proving public transcripts contain no graph fields.
- Kani or equivalent harness for no-allocation/no-global-retention admission.
- Static tripwires for forbidden raw-envelope traits and owned-copy helpers.
- Sovereign Hardened attestation evidence before hardware-backed claims.

## Rejected Alternatives

- Treating encryption-at-rest as graph non-publication.
- Using LEANN/vector indexes as a consensus or sharding primitive.
- Allowing production WALs to persist raw transaction bytes for convenience.
- Making DPU/SmartNIC hardware mandatory for every deployment profile.
- Claiming generic subpoena immunity or impossible recovery without a declared
  threat model and profile evidence.
