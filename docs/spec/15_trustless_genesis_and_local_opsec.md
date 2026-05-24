# QASH Trustless Genesis and Vendor-Agnostic Local Hardware OpSec
## `docs/spec/15_trustless_genesis_and_local_opsec.md` — Anti-Ceremony Invariant Draft

> **Status:** Derived engineering specification. This document constrains future
> Domain B hardware hardening work and clarifies that hardware security is local
> operator OpSec, not protocol authority or a vendor mandate.

---

## §15.1 — Purpose

This document prevents QASH from drifting into enterprise PKI, foundation
custody, trusted setup, ceremony-based bootstrapping, or vendor-specific hardware
requirements.

QASH is designed to be a governance-free deterministic state-transition
substrate. Its genesis and liveness properties must not depend on private
humans, vendor hardware, custody ceremonies, trusted dealers, or procurement
choices.

---

## §15.2 — Trustless Genesis Axiom

Genesis is a deterministic mathematical event.

The genesis artifact set may include:

- locked specification hashes,
- source and build artifact hashes,
- genesis constants,
- public entropy references,
- reproducible-build evidence,
- proof-object and traceability manifests.

The genesis artifact set MUST NOT require:

- a foundation or governance signature,
- a trusted setup ceremony,
- multi-party toxic-waste generation,
- private validator whitelists,
- vendor hardware attestations,
- human quorum approval,
- network master keys,
- any private secret that must be trusted by later participants.

Anyone must be able to independently reconstruct and verify genesis from public
artifacts.

---

## §15.3 — Anti-Ceremony Rules

### AC-1: No trusted setup

QASH MUST use transparent cryptographic assumptions only for protocol genesis.
No CRS ceremony, toxic-waste ceremony, trusted dealer, or hidden setup secret is
permitted.

### AC-2: No genesis cabal

No person, organization, foundation, validator set, hardware vendor, or custody
committee may possess special authority to create, approve, unlock, or validate
the network genesis state.

### AC-3: No protocol-critical vendor hardware

No hardware authenticator, smart card, HSM, TPM, DPU, TEE, cloud KMS, sovereign
secure element, or vendor-specific device may be required to run a conforming
QASH node in the permissionless profile.

### AC-4: No human liveness dependency

Routine transaction admission, epoch closure, finality, replay, halt handling,
and global liveness recovery MUST NOT require physical touch, PIN entry, smart
card insertion, human quorum, manual ceremony, HSM-admin approval, or cloud-KMS
approval.

### AC-5: No hardware-derived consensus semantics

Hardware attestation, hardware identifiers, HSM/KMS audit logs, FIDO assertions,
PIV certificates, TPM quotes, enclave reports, secure-element state, or local
operator policies MUST NOT change Domain A state transitions, halt semantics,
finality rules, or public transcript semantics.

### AC-6: Standards before vendors

Normative QASH specifications MUST describe optional hardware integration by
standards, interface classes, and evidence classes rather than vendor-specific
products.

Vendor-specific OIDs, CLI commands, object schemas, audit-log formats, daemon
names, and SDK assumptions belong in optional backend guides or conformance
profiles, not in the protocol authority layer.

---

## §15.4 — Permitted Local Hardware OpSec

Hardware security is permitted as optional local node protection.

Permitted Domain B uses include:

- FIDO2/CTAP2/WebAuthn or equivalent for operator SSH and administrative access,
- PIV/FIPS 201 or equivalent smart-card profiles for local administrative keys,
- PKCS#11 or equivalent HSM interfaces for local validator key wrapping,
- TPM 2.0 or equivalent measured-boot attestation for local platform evidence,
- FIPS 140-3 or sovereign-equivalent HSMs for local custody requirements,
- HSM-backed or KMS-backed local audit MAC chains,
- release-signing keys for a specific distributor or maintainer,
- local backup and disaster recovery,
- optional threshold signing for one operator's own node key,
- hardware-backed erasure evidence for one operator's local storage.

These controls may improve local security. They do not become network security
assumptions. Their failure affects only the local operator.

---

## §15.5 — Vendor-Agnostic Hardware Boundary

Hardware components may be useful in QASH deployments, but only within this local
OpSec boundary:

| Capability class | Permitted role | Forbidden role |
|---|---|---|
| FIDO2/CTAP2/WebAuthn authenticator | operator login, admin approval, local recovery | consensus signing, epoch finality, genesis authority |
| PIV/FIPS 201 smart card or equivalent | local release signing, local recovery, optional maintainer keys | protocol master key, required genesis ceremony |
| PKCS#11/FIPS/sovereign HSM | local wrap keys, audit MACs, opaque local objects | network master key, consensus halt oracle |
| TPM 2.0 or equivalent platform attestor | local measured-boot evidence | Domain A input, permissionless validator requirement |
| TEE/DPU | optional local hardening for admission/key handling | protocol authority, genesis dependency |
| Hardware audit log | local custody evidence | Domain A halt trigger or global liveness condition |

No vendor product is treated as a native requirement for QASH. Yubico, Nitrokey,
Google Titan, Feitian, Thales, Utimaco, Entrust, Infineon, AWS CloudHSM, Azure
Managed HSM, national/sovereign HSMs, or equivalent devices may appear in
operator guides as backend examples only.

PQC hot-path execution belongs in software, TEE, DPU, or future PQC-capable local
hardware profiles. It remains Domain B local deployment infrastructure and must
not become a permissionless protocol requirement.

---

## §15.6 — Public Transcript Privacy Rule

Raw hardware identifiers are forbidden in global public transcript surfaces.

Forbidden public fields include:

- hardware-authenticator serial numbers,
- FIDO AAGUIDs,
- TPM endorsement keys,
- HSM or smart-card serial numbers,
- certificate subject names,
- cloud KMS resource identifiers,
- enclave measurement metadata tied to a persistent operator identity,
- stable device fingerprints.

Permissioned or sovereign deployments may derive blinded fingerprints for
private Domain B registries:

```text
hardware_fingerprint = H(profile_salt || device_identifier_set)
```

Such fingerprints remain deployment metadata. They are not Domain A inputs and
are not global public transcript fields unless a future privacy-reviewed spec
explicitly admits them.

---

## §15.7 — Failure Semantics

Local hardware failure must remain local.

| Failure | Permitted consequence | Forbidden consequence |
|---|---|---|
| Lost authenticator or smart card | operator loses local admin/recovery path | network halt |
| HSM/KMS log full or unavailable | local alert/quarantine | Domain A absorbing halt |
| TEE/DPU/TPM attestation mismatch | local node not admitted to private profile | global genesis rejection |
| Threshold signer unavailable | operator's validator may miss signing | global finality dependency |
| HSM/KMS erased local key | local key unavailable | protocol state mutation |

---

## §15.8 — Review Gate

Future specs, ADRs, roadmap items, and implementation PRs must be rejected or
rewritten if they introduce:

- trusted setup,
- genesis ceremonies,
- foundation-controlled network keys,
- human quorums for protocol liveness,
- vendor hardware as a protocol requirement,
- local HSM/KMS/TEE/TPM/DPU state as a Domain A input,
- raw hardware identifiers in public transcript surfaces.

This review gate applies even when the proposal improves local security.
