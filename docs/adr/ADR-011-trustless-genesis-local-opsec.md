# ADR-011: Trustless Genesis and Vendor-Agnostic Local Hardware OpSec

**Status:** Proposed  
**Date:** 2026-05-24  
**Source:** QASH ethos realignment and hardware adoptability review  
**PDF authority:** PDF-SILENT

## Context

QASH is designed as a governance-free, ceremony-free, deterministic execution
substrate. Recent Domain B hardening discussions introduced useful hardware-root
concepts from specific vendor ecosystems. Some language drifted toward enterprise
PKI and trusted-ceremony assumptions: genesis ceremonies, master key custodians,
validator cabals, network-level hardware authority, and vendor-specific devices.

That framing conflicts with QASH's foundational ethos:

- no trusted setup,
- no genesis cabal,
- no foundation-controlled master keys,
- no hardware device with protocol authority,
- no human-in-the-loop operation in consensus or liveness paths,
- no vendor lock-in for optional local security hardening.

Hardware may improve local node security. It must not become a protocol trust
anchor, adoption barrier, or procurement mandate.

## Decision

QASH adopts a **Trustless Genesis and Vendor-Agnostic Local Hardware OpSec**
invariant.

Hardware-backed authenticators, smart cards, HSMs, TPMs, TEEs, DPUs,
cloud/enterprise KMS systems, FIDO2/PIV devices, and threshold-signing systems
are optional Domain B deployment tools. They may protect one operator's local
keys, release process, SSH access, daemon controls, or audit logs. They have zero
protocol-level authority and must not alter Domain A state transitions, genesis
validity, validator admission, epoch finality, halt logic, or public transcript
semantics.

Vendor products such as YubiKey or YubiHSM may be used as reference
implementations in deployment guides, but normative requirements must be stated
in terms of interoperable standards and capability classes such as FIDO2/CTAP2,
WebAuthn, PIV/FIPS 201, PKCS#11, TPM 2.0, FIPS 140-3 or sovereign-equivalent HSM
profiles, and authenticated encrypted secure-element sessions.

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
foundation keyholders, master-key custodians, private multi-sig approvals, vendor
hardware attestations, or HSM-held network secrets.

### TG-3: Hardware is local OpSec only

Hardware security modules and security keys are local deployment choices. If a
hardware authenticator, smart card, HSM, TPM, TEE, DPU, cloud KMS, or
threshold-signing cluster is destroyed, lost, seized, revoked, unavailable, or
misconfigured, only the local operator's node or release process is affected. The
network MUST NOT halt, fork, reject genesis, or change consensus state because of
local hardware state.

### TG-4: No hardware state in Domain A

Domain A MUST NOT import, parse, hash for semantic effect, or branch on:

- X.509, ASN.1, PIV, FIDO2, CTAP, U2F, WebAuthn, PKCS#11, TPM, or KMS artifacts,
- TPM quotes, endorsement keys, attestation certificates, PCR values, or enclave reports,
- HSM logs, domains, capabilities, object IDs, serials, or auth state,
- hardware serial numbers, AAGUIDs, device fingerprints, or operator identities,
- HSM/KMS fault states, audit-log fullness, secure-element failures, or admin events.

Domain A may only receive scalar commitments and admissible protocol inputs
specified elsewhere.

### TG-5: No human liveness dependency

Physical touch, PIN entry, operator quorum, smart-card insertion, release
approval, HSM-admin action, KMS approval, or secure-element policy check MUST NOT
be required for routine transaction admission, epoch closure, finality, replay,
halt handling, or recovery of global liveness.

Human-gated operations may exist only for local administration: SSH, daemon
restart, release signing, backup recovery, operator key rotation, or local node
custody actions.

### TG-6: No public hardware identifiers

Raw hardware-authenticator serials, AAGUIDs, TPM endorsement keys, HSM serials,
certificate subjects, enclave measurements tied to persistent identities, or
stable device fingerprints MUST NOT be included in global public transcripts.

Permissioned or sovereign deployments may use blinded hardware fingerprints in
private Domain B registries, but those fingerprints must remain outside Domain A
and outside the global public transcript unless separately admitted by a future
privacy-reviewed spec.

### TG-7: Standards before vendors

Normative QASH documents MUST define hardware integration by standards,
interfaces, and evidence classes rather than specific vendor products.

Permitted normative references include:

- FIDO2/CTAP2/WebAuthn for operator authentication,
- PIV/FIPS 201 or equivalent smart-card profiles for local administrative keys,
- PKCS#11 or equivalent HSM interfaces for local key custody,
- TPM 2.0 or equivalent measured-boot attestation for local platform evidence,
- FIPS 140-3 or jurisdictional sovereign-equivalent validation levels for HSMs,
- mutually authenticated encrypted channels or secure-element sessions for host
  to hardware communication.

Vendor-specific OIDs, CLI commands, object layouts, and audit-log formats belong
in backend guides or conformance profiles, not in the protocol authority layer.

## Permitted Local Hardware Uses

Hardware-backed security tooling may be used for:

- local SSH/FIDO2/WebAuthn administrator authentication,
- local daemon restart authorization,
- local release-signing keys,
- local validator key custody,
- local wrap keys and opaque objects,
- local audit MAC chains,
- local key-erasure evidence,
- local backup/recovery procedures,
- optional operator-side threshold redundancy.

These uses are Domain B only and optional.

## Reference Backends, Not Protocol Requirements

Examples of possible local OpSec backends include:

- hardware security keys implementing FIDO2/CTAP2/WebAuthn,
- PIV/FIPS 201 smart cards or equivalent national smart-card profiles,
- PKCS#11-accessible HSMs,
- TPM 2.0 devices,
- cloud HSM/KMS systems used only for local operator custody,
- sovereign-certified HSMs required by a particular jurisdiction,
- YubiKey, YubiHSM, Nitrokey, Google Titan, Feitian, Thales, Utimaco, Entrust,
  AWS CloudHSM, Azure Managed HSM, Infineon, or other equivalent devices.

Listing an example does not make it a protocol dependency or endorsement.

## Rejected Hardware Uses

QASH rejects:

- vendor devices as genesis authorities,
- HSM-held network master keys,
- hardware-gated validator admission in the permissionless profile,
- FIDO/PIV touch or PIN on the consensus hot path,
- HSM/KMS audit-log divergence mapped to Domain A `HaltReason`,
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
- Avoids narrowing adoption to one vendor ecosystem.
- Lets regulated operators use existing FIPS, PKCS#11, TPM, PIV, or sovereign
  hardware investments.
- Preserves Domain A purity and liveness independence.

### Cost

- Some compliance deployments must document local hardware controls separately
  rather than representing them as protocol guarantees.
- Hardware-backed validator identity is limited to permissioned/private Domain B
  registries and cannot be a permissionless network requirement.
- Release and operator security improvements remain operational evidence, not
  consensus evidence.
- Backend guides must handle vendor-specific behavior outside normative specs.

## Required Follow-up

- Add a Trustless Genesis / Anti-Ceremony invariant to the execution model or
  traceability docs.
- Add a vendor-agnostic local hardware OpSec guide with optional backend profiles.
- Ensure roadmap hardware-root language is scoped to Domain B local deployment.
- Add CI/document-hygiene checks or review rules rejecting new spec text that
  makes trusted ceremonies, human quorums, vendor hardware, or local hardware
  state protocol-critical.
