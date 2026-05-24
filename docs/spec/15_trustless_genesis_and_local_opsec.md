# QASH Trustless Genesis and Local Hardware OpSec
## `docs/spec/15_trustless_genesis_and_local_opsec.md` — Anti-Ceremony Invariant Draft

> **Status:** Derived engineering specification. This document constrains future
> Domain B hardware hardening work and clarifies that hardware security is local
> operator OpSec, not protocol authority.

---

## §15.1 — Purpose

This document prevents QASH from drifting into enterprise PKI, foundation
custody, trusted setup, or ceremony-based bootstrapping models.

QASH is designed to be a governance-free deterministic state-transition
substrate. Its genesis and liveness properties must not depend on private
humans, vendor hardware, custody ceremonies, or trusted dealers.

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
- YubiKey, YubiHSM, TPM, HSM, DPU, or TEE attestations,
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

No Yubico, HSM, TPM, DPU, TEE, smart card, cloud KMS, or equivalent vendor device
may be required to run a conforming QASH node in the permissionless profile.

### AC-4: No human liveness dependency

Routine transaction admission, epoch closure, finality, replay, halt handling,
and global liveness recovery MUST NOT require physical touch, PIN entry, smart
card insertion, human quorum, manual ceremony, or admin approval.

### AC-5: No hardware-derived consensus semantics

Hardware attestation, hardware identifiers, HSM audit logs, FIDO assertions,
PIV certificates, TPM quotes, enclave reports, or local operator policies MUST
NOT change Domain A state transitions, halt semantics, finality rules, or public
transcript semantics.

---

## §15.4 — Permitted Local Hardware OpSec

Hardware security is permitted as optional local node protection.

Permitted Domain B uses include:

- FIDO2 or PIV for operator SSH and administrative access,
- YubiHSM or equivalent HSM for local validator key wrapping,
- HSM-backed local audit MAC chains,
- release-signing keys for a specific distributor or maintainer,
- local backup and disaster recovery,
- optional threshold signing for one operator's own node key,
- hardware-backed erasure evidence for one operator's local storage.

These controls may improve local security. They do not become network security
assumptions. Their failure affects only the local operator.

---

## §15.5 — Yubico Integration Boundary

Yubico components may be useful in QASH deployments, but only within this local
OpSec boundary:

| Component | Permitted role | Forbidden role |
|---|---|---|
| YubiKey FIDO2 | operator login, admin approval, local recovery | consensus signing, epoch finality, genesis authority |
| YubiKey PIV | local release signing, local recovery, optional maintainer keys | protocol master key, required genesis ceremony |
| YubiHSM 2 | local wrap keys, audit MACs, opaque local objects | network master key, PQC hot-path engine, consensus halt oracle |
| FIDO2 Enterprise Attestation | private permissioned inventory evidence | global public transcript identity |
| YubiHSM audit log | local custody evidence | Domain A halt trigger or global liveness condition |

YubiHSM 2 is not treated as a native ML-KEM or ML-DSA execution engine. PQC
hot-path execution belongs in software, TEE, DPU, or a future PQC-capable HSM
profile that remains Domain B local deployment infrastructure.

---

## §15.6 — Public Transcript Privacy Rule

Raw hardware identifiers are forbidden in global public transcript surfaces.

Forbidden public fields include:

- YubiKey serial numbers,
- FIDO AAGUIDs,
- TPM endorsement keys,
- HSM serial numbers,
- certificate subject names,
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
| Lost YubiKey | operator loses local admin/recovery path | network halt |
| YubiHSM log full | local alert/quarantine | Domain A absorbing halt |
| TEE attestation mismatch | local node not admitted to private profile | global genesis rejection |
| Threshold signer unavailable | operator's validator may miss signing | global finality dependency |
| HSM erased local key | local key unavailable | protocol state mutation |

---

## §15.8 — Review Gate

Future specs, ADRs, roadmap items, and implementation PRs must be rejected or
rewritten if they introduce:

- trusted setup,
- genesis ceremonies,
- foundation-controlled network keys,
- human quorums for protocol liveness,
- vendor hardware as a protocol requirement,
- local HSM/TEE state as a Domain A input,
- raw hardware identifiers in public transcript surfaces.

This review gate applies even when the proposal improves local security.
