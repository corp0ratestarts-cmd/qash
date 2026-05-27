# Accelerator and Hardware Evidence Profiles

**Status:** Pre-genesis planning document.  
**Parent:** [`authorized_platform_matrix.md`](authorized_platform_matrix.md) — Tier D profiles.

---

## Core rule

> **Accelerators and hardware security devices may be used for proving, verification acceleration, simulation, evidence generation, indexing, or operator tooling. They must not become authoritative Domain A consensus execution engines. They must not change Domain A state-root semantics, introduce accelerator scheduling into consensus, or alter replay equivalence.**

Any accelerator or hardware integration belongs in Domain B (`qash-pal`) and passes only validated scalar effects, commitments, or bounded evidence handles into Domain A. The Domain A state root is defined entirely by `qash-consensus` running on CPU under the deterministic `no_std` replay kernel — not by any GPU, TPU, FPGA, HSM, TPM, or TEE.

Evidence required before any profile is listed as supported: see [`authorized_platform_matrix.md` — Evidence levels](authorized_platform_matrix.md#evidence-levels).

---

## GPU / Compute Accelerators

### Moore Threads / MUSA

**Vendor:** Moore Threads Technology (China)  
**ISA:** MUSA (Moore Unified System Architecture)  
**Typical devices:** MTT S30, MTT S60, MTT S70, MTT S80  
**SDK:** MUSA SDK (mcc compiler, libmusart, thrust-like APIs)

#### Permitted Domain B uses

| Use case | Description |
|----------|-------------|
| ZK proving acceleration | Batch PLONK / Groth16 / FRI-STARK witness generation and proof computation |
| Proof aggregation | Parallel recursive proof aggregation for epoch proofs |
| Batch verification | Parallel Dilithium5 / SLH-DSA / Falcon-512 batch signature verification |
| Evidence generation | Parallel SHA3-256 / SM3-256 hash computation for evidence artifact production |
| Simulation | Epoch-transition simulation at scale for pre-genesis testing |

#### Forbidden uses

- Executing the Domain A state machine on MUSA hardware as an authoritative consensus node.
- Using MUSA random number generation in any path that feeds Domain A.
- Using MUSA timing APIs to gate Domain A epoch transitions.
- Exposing MUSA kernel outputs directly as Domain A state root values.

#### Domain B integration path

```
qash-pal
  └── musa/
      ├── prover.rs        — ZK proving via MUSA compute kernels (Domain B)
      ├── verifier.rs      — batch signature verification (Domain B)
      └── evidence.rs      — parallel hash computation for artifacts (Domain B)
```

MUSA results flow into Domain B as opaque commitment bytes. Domain A never imports MUSA types. The `audit_domain_boundary_full.sh` script enforces this via the `musa::` import pattern check.

#### Sovereign hardware posture

Moore Threads/MUSA is strategically significant for deployments requiring China-origin or sovereign hardware independence. This profile is recorded as a planned Tier D evidence target for that reason.

#### Evidence milestone

L1: A Domain B crate (`qash-pal-musa`) compiles against the MUSA SDK and produces a commitment output verifiable by Domain A without importing any MUSA types into `qash-consensus`.

#### Non-claims

QASH does not currently have a MUSA PAL integration. MUSA support is a planned evidence target.

---

### NVIDIA CUDA

**Vendor:** NVIDIA  
**SDK:** CUDA Toolkit, cuSPARSE, cuBLAS, Thrust

#### Permitted Domain B uses

ZK proving (cuZK / custom CUDA kernels), batch signature verification, simulation, evidence generation, operator tooling.

#### Forbidden uses

Same as MUSA: no CUDA RNG, no CUDA timing, no CUDA kernel output as authoritative Domain A state root.

#### Domain B integration path

```
qash-pal
  └── cuda/
      ├── prover.rs        — ZK proving via CUDA (Domain B)
      └── verifier.rs      — batch verification (Domain B)
```

Uses `cust` or `rust-cuda` Rust bindings. All CUDA types stay in Domain B.

#### Non-claims

QASH does not currently have a CUDA PAL integration. CUDA support is a planned evidence target.

---

### AMD ROCm

**Vendor:** AMD  
**SDK:** ROCm (Radeon Open Compute), HIP, hipSPARSE

#### Permitted Domain B uses

ZK proving, batch verification, simulation, evidence generation.

#### Domain B integration path

HIP is CUDA-compatible at the API level; the CUDA integration path applies with `hip` replacing `cuda` in crate names.

#### Non-claims

QASH does not currently have a ROCm PAL integration. ROCm support is a planned evidence target.

---

### Intel oneAPI / Level Zero

**Vendor:** Intel  
**SDK:** Intel oneAPI, SYCL, Level Zero

#### Permitted Domain B uses

ZK proving on Intel Arc/Xe, batch verification, operator tooling.

#### Non-claims

QASH does not currently have an oneAPI PAL integration. oneAPI support is a planned evidence target.

---

### Vulkan Compute

**Standard:** Khronos Vulkan 1.3 (compute pipeline)  
**Applicability:** Cross-vendor GPU compute on any Vulkan-capable device

#### Permitted Domain B uses

Cross-vendor ZK proving (portable GLSL/SPIR-V compute shaders), batch hash computation, simulation.

#### Non-claims

QASH does not currently have a Vulkan compute PAL integration. Vulkan compute support is a planned evidence target.

---

### OpenCL

**Standard:** Khronos OpenCL 3.0  
**Applicability:** Legacy and embedded GPU/FPGA compute

#### Permitted Domain B uses

Batch hash computation, FPGA-accelerated verification on OpenCL-capable devices.

#### Non-claims

QASH does not currently have an OpenCL PAL integration. OpenCL support is a planned evidence target.

---

### Apple Metal

**Vendor:** Apple  
**SDK:** Metal Performance Shaders (MPS), Metal compute pipelines

#### Permitted Domain B uses

ZK proving on Apple Silicon (M-series), batch verification, macOS/iOS operator tooling.

#### Non-claims

QASH does not currently have a Metal PAL integration. Metal support is a planned evidence target.

---

## Hardware Security / Attestation Devices

### TPM 2.0

**Standard:** TCG TPM 2.0  
**Applicability:** x86 (fTPM, dTPM), ARM (Pluton, firmware TPM), embedded

#### Permitted Domain B uses

| Use case | TPM API |
|----------|---------|
| Platform attestation | `TPM2_Quote`, `TPM2_Certify` — attests platform state before consensus start |
| Key sealing | `TPM2_Seal` / `TPM2_Unseal` — seals epoch keys to platform PCR values |
| Evidence signing | `TPM2_Sign` — signs evidence artifacts with TPM-resident keys |
| Random entropy | `TPM2_GetRandom` — entropy for Domain B key generation only |

#### Forbidden uses

- Using TPM `TPM2_GetRandom` output as input to Domain A computation (entropy ingress violation).
- Using PCR values directly in Domain A state-root arithmetic.
- Treating a TPM `TPM2_Quote` as an authoritative Domain A state-root commitment.

#### Domain B integration path

```
qash-pal
  └── tpm/
      ├── attestation.rs   — TPM2_Quote / TPM2_Certify (Domain B)
      ├── key_seal.rs      — TPM2_Seal / TPM2_Unseal (Domain B)
      └── evidence.rs      — TPM2_Sign for artifact signing (Domain B)
```

Uses `tss-esapi` (Rust TPM2 TSS binding). All TPM types stay in Domain B. TPM outputs are opaque commitment bytes or attestation handles — they do not enter Domain A arithmetic.

#### Non-claims

QASH does not currently have a TPM PAL integration. TPM 2.0 support is a planned evidence target.

---

### HSM / PKCS#11

**Standard:** OASIS PKCS#11 v3.0  
**Applicability:** Network HSMs (Thales Luna, AWS CloudHSM, Utimaco), USB HSMs (YubiHSM 2), PKCS#11 tokens

#### Permitted Domain B uses

| Use case | PKCS#11 mechanism |
|----------|------------------|
| Key storage | `C_GenerateKeyPair`, `C_CreateObject` — keys never leave the HSM |
| Signing | `C_Sign` with Dilithium5 / ECDSA / RSA (mechanism-dependent) |
| Verification | `C_Verify` for batch signature verification |
| Attestation | Vendor attestation APIs (mechanism varies by HSM model) |

#### Forbidden uses

- Using HSM random output (`C_GenerateRandom`) in Domain A computation.
- Using HSM-computed digests directly as Domain A state-root values.
- Treating HSM key handles as Domain A identifiers.

#### Domain B integration path

```
qash-pal
  └── hsm/
      ├── pkcs11.rs        — C_Sign / C_Verify / C_GenerateKeyPair (Domain B)
      └── attestation.rs   — vendor attestation APIs (Domain B)
```

Uses `pkcs11` or `cryptoki` Rust crates. All PKCS#11 handles and CK_SESSION_HANDLE values stay in Domain B.

#### Non-claims

QASH does not currently have an HSM/PKCS#11 PAL integration. HSM support is a planned evidence target.

---

### Smartcard / JavaCard

**Standard:** ISO 7816, GlobalPlatform GP 2.3, JavaCard 3.1  
**Applicability:** Contact/contactless smartcards, SIM/eSIM, secure elements

#### Permitted Domain B uses

Operator credential storage, selective disclosure key management, pilot evidence signing on secure elements.

#### Forbidden uses

Using smartcard-computed values as Domain A state-root inputs.

#### Domain B integration path

JavaCard applets or native ISO 7816 APDU exchange via `pcsc` Rust crate. All smartcard types stay in Domain B.

#### Non-claims

QASH does not currently have a smartcard/JavaCard PAL integration. Smartcard support is a planned evidence target.

---

### TEE / TrustZone (Arm)

**Standard:** Arm TrustZone, OP-TEE (open-source TEE OS), GlobalPlatform TEE API  
**Applicability:** Arm Cortex-A (mobile, server, embedded), Arm Cortex-M (TrustZone-M)

#### Permitted Domain B uses

| Use case | TEE mechanism |
|----------|---------------|
| Local evidence generation | TEE-attested hash of Domain B state — commitment only |
| Key isolation | Private keys stored in secure world; never exposed to normal world |
| Selective disclosure | TEE-sealed disclosure keys; Domain B handles sealing/unsealing |
| Attestation report | `TEEC_InvokeCommand` to a trusted application that produces an attestation commitment |

#### Forbidden uses

- Running Domain A consensus in the TEE secure world as the authoritative execution path.
- Using TEE-generated random values as Domain A inputs.
- Exposing TEE attestation reports as Domain A state roots.

#### Domain B integration path

```
qash-pal
  └── tee/
      ├── optee.rs         — OP-TEE TEEC_InvokeCommand wrappers (Domain B)
      └── attestation.rs   — TEE attestation commitment production (Domain B)
```

Uses `optee-teec` or vendor SDK bindings. TEE output is an opaque attestation commitment consumed by Domain B, not Domain A.

#### Non-claims

QASH does not currently have a TrustZone/TEE PAL integration. TEE support is a planned evidence target.

---

### SGX-Style Enclave (reference profile)

**Standard:** Intel SGX (reference architecture); AMD SEV-SNP as comparable confidential-compute model  
**Applicability:** x86 server platforms with confidential compute support

#### Permitted Domain B uses

Confidential compute for operator key material, evidence generation in an enclave, remote attestation reports as Domain B commitments.

#### Forbidden uses

Running Domain A consensus inside an SGX enclave as the authoritative execution — this would make replay verification enclave-dependent and break cross-ISA determinism guarantees.

#### Non-claims

QASH does not currently have an SGX PAL integration. SGX-style enclave support is a planned evidence target.

---

## Common requirements for all Tier D integrations

Every Tier D PAL adapter added to `qash-pal` must satisfy:

1. **No Domain A import**: the adapter crate must not import or re-export any type from `qash-consensus`. The `audit_domain_boundary_full.sh` script enforces this.
2. **Commitment boundary**: accelerator / hardware outputs that cross the Domain A/B boundary must be opaque fixed-width commitment bytes (e.g., 32-byte hash), not raw accelerator-specific types.
3. **No entropy ingress**: random values from GPUs, TPMs, HSMs, or TEEs must never enter Domain A computation. Domain A is deterministic and replay-invariant.
4. **No timing ingress**: accelerator or hardware timing values must never gate Domain A epoch transitions.
5. **Replay invariance**: Domain A state roots must be identical whether or not a Tier D accelerator is present. Accelerators are a Domain B performance and evidence concern only.
6. **Audit trail**: each Tier D integration must produce a verifiable evidence artifact (L5) before any support claim is made.

---

## Non-claims boundary

The following statements are **not** authorised claims at this stage:

- QASH accelerates proving with Moore Threads / MUSA / CUDA / ROCm / oneAPI / Vulkan / OpenCL / Metal.
- QASH integrates with HSMs / TPMs / smartcards / TEEs / SGX.
- Domain A consensus runs on any GPU or hardware security device.
- QASH is hardware-attested by any TPM, HSM, TEE, or smartcard.

Authorised design-intent language:

> QASH is designed for a Domain B accelerator and hardware evidence profile universe spanning GPU compute (Moore Threads/MUSA, CUDA, ROCm, oneAPI, Vulkan, OpenCL, Metal) and hardware security devices (TPM 2.0, HSM/PKCS#11, smartcard/JavaCard, TEE/TrustZone, SGX-style enclave). All Tier D profiles are Domain B concerns. They must not alter Domain A state-root semantics, introduce non-determinism into consensus, or change replay equivalence. Support claims are evidence-gated and require L3 replay evidence at minimum.
