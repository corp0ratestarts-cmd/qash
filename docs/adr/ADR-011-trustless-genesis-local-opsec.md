# ADR-011: Trustless Genesis and Local Hardware OpSec

**Status:** Proposed  
**Date:** 2026-05-24  
**Source:** QASH ethos realignment review  
**PDF authority:** PDF-SILENT

## Context

QASH is designed as a governance-free, ceremony-free, deterministic execution
substrate. Recent Domain B hardening discussions introduced useful hardware-root
concepts from Yubico/HSM ecosystems, but some language drifted toward enterprise
PKI and trusted-ceremony assumptions: genesis ceremonies, master key custodians,
validator cabals, and network-level hardware authority.

That framing conflicts with QASH's foundational ethos:

- no trusted setup,
- no genesis cabal,
- no foundation-controlled master keys,
- no hardware device with protocol authority,
- no human-in-the-loop operation in consensus or liveness paths.

Hardware may improve local node security. It must not become a protocol trust
anchor.

## Decision

QASH adopts a **Trustless Genesis and Local Hardware OpSec** invariant.

YubiKey, YubiHSM, TPM, HSM, TEE, DPU, FIDO2, PIV, and threshold-signing systems
are optional Domain B deployment tools. They may protect one operator's local
keys, release process, SSH access, or audit logs. They have zero protocol-level
authority and must not alter Domain A state transitions, genesis validity,
validator admission, epoch finality, halt logic, or public transcript semantics.

## Normative Rules

### TG-1: No trusted setup

QASH MUST NOT require a cryptographic trusted setup, toxic-waste ceremony,
Common Reference String ceremony, foundation-generated secret, or any equivalent
human coordination event to bootstrap or maintain the network.

Transparent proof systems, hash commitments, deterministic code hashes, and
public entropy are admissible. Secret ceremony material is not.

### TG-2: No genesis cabal

Genesis is a deterministic mathematical event, not a human ceremony.

The genesis artifact set may include locked specs, code hashes, genesis constants,
public entropy references, and reproducible-build evidence. It MUST NOT depend on
foundation keyholders, master-key custodians, private multi-sig approvals, or
Yubico/HSM attestations.

### TG-3: Hardware is local OpSec only

Hardware security modules and security keys are local deployment choices. If a
YubiKey, YubiHSM, TPM, HSM, TEE, DPU, or threshold-signing cluster is destroyed,
lost, seized, or misconfigured, only the local operator's node or release process
is affected. The network MUST NOT halt, fork, reject genesis, or change consensus
state because of local hardware state.

### TG-4: No hardware state in Domain A

Domain A MUST NOT import, parse, hash for semantic effect, or branch on:

- X.509, ASN.1, PIV, FIDO2, CTAP, U2F, or WebAuthn artifacts,
- TPM quotes, endorsement keys, attestation certificates, or PCR values,
- YubiHSM logs, domains, capabilities, object IDs, serials, or auth state,
- hardware serial numbers, AAGUIDs, device fingerprints, or operator identities,
- HSM fault states, audit-log fullness, Force Audit failures, or admin events.

Domain A may only receive scalar commitments and admissible protocol inputs
specified elsewhere.

### TG-5: No human liveness dependency

Physical touch, PIN entry, operator quorum, smart-card insertion, release
approval, or HSM-admin action MUST NOT be required for routine transaction
admission, epoch closure, finality, replay, halt handling, or recovery of global
liveness.

Human-gated operations may exist only for local administration: SSH, daemon
restart, release signing, backup recovery, operator key rotation, or local node
custody actions.

### TG-6: No public hardware identifiers

Raw YubiKey serials, AAGUIDs, TPM endorsement keys, HSM serials, certificate
subjects, or persistent device fingerprints MUST NOT be included in global public
transcripts.

Permissioned or sovereign deployments may use blinded hardware fingerprints in
private Domain B registries, but those fingerprints must remain outside Domain A
and outside the global public transcript unless separately admitted by a future
privacy-reviewed spec.

## Permitted Yubico/HSM Uses

Yubico and HSM tooling may be used for:

- local SSH/FIDO2 administrator authentication,
- local daemon restart authorization,
- release-signing keys,
- local validator key custody,
- local wrap keys and opaque objects,
- local audit MAC chains,
- local key-erasure evidence,
- local backup/recovery procedures,
- optional operator-side threshold redundancy.

These uses are Domain B only and optional.

## Rejected Yubico/HSM Uses

QASH rejects:

- YubiKey/PIV/FIDO2 as a genesis authority,
- HSM-held network master keys,
- hardware-gated validator admission in the permissionless profile,
- FIDO touch or PIN on the consensus hot path,
- HSM audit-log divergence mapped to Domain A `HaltReason`,
- raw hardware identifiers in `PublicTranscript`,
- any claim that possession of a specific vendor device is required to run QASH.

## Relationship to ADR-010

ADR-010 defines zero-persistence Domain B admission. This ADR constrains the
hardware part of that profile: hardware may harden local deployment, but it does
not create protocol trust. `sovereign-hardened` remains an optional deployment
profile, not a baseline requirement and not a source of consensus authority.

## Consequences

### Positive

- Restores QASH's no-trusted-ceremony ethos.
- Prevents enterprise PKI assumptions from leaking into the protocol.
- Keeps Yubico/HSM integrations useful without centralizing the network.
- Preserves Domain A purity and liveness independence.

### Cost

- Some compliance deployments must document local hardware controls separately
  rather than representing them as protocol guarantees.
- Hardware-backed validator identity is limited to permissioned/private Domain B
  registries and cannot be a permissionless network requirement.
- Release and operator security improvements remain operational evidence, not
  consensus evidence.

## Required Follow-up

- Add a Trustless Genesis / Anti-Ceremony invariant to the execution model or
  traceability docs.
- Add a local-only hardware OpSec guide for optional Yubico/HSM use.
- Ensure roadmap hardware-root language is scoped to Domain B local deployment.
- Add CI/document-hygiene checks or review rules rejecting new spec text that
  makes trusted ceremonies, human quorums, or vendor hardware protocol-critical.
