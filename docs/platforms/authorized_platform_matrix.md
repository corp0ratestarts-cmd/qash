# Authorised Platform Matrix

**Status:** Pre-genesis planning document.  
**Governing principle:** Domain A is a universal deterministic replay kernel. All platform-specific runtimes, operating systems, RTOSes, accelerators, and hardware evidence subsystems are Domain B concerns and must not alter Domain A state-root semantics.

---

## Evidence-gating rule

> Current TH-7 CI covers Linux x86\_64/aarch64/riscv64gc only. Additional CPU/ISA, OS, RTOS, accelerator, and hardware profiles are planned evidence targets. No broader support claim is allowed until compile, replay, and evidence artifacts exist for that tier.

No platform may be marked as "supported" without:

1. A passing `cargo check` or `cargo build` artifact (Evidence Level 1).
2. A canonical state-root match against the native x86\_64 reference (Evidence Level 3).
3. A signed/indexed evidence snapshot committed to `artifacts/evidence/` (Evidence Level 5).

Intermediate levels are advisory only and do not constitute a support claim.

---

## Evidence levels

| Level | Meaning |
|-------|---------|
| L1 | `cargo check` / `cargo build` succeeds for the target triple |
| L2 | Unit tests pass under the target (QEMU, cross, or native) |
| L3 | Canonical state root matches the native x86\_64 reference root |
| L4 | v1.1 and v1.2 replay corpus vectors match pinned roots |
| L5 | Evidence artifact captured, committed, and indexed |

---

## Tier A — Genesis-blocking

These targets are blocking requirements enforced by `platform-determinism.yml`.  
State-root parity failures on Tier A targets block all CI.

| Target triple | Runner / execution | Current CI status |
|---------------|-------------------|-------------------|
| `x86_64-unknown-linux-gnu` | GitHub hosted (ubuntu-latest), native | ✅ Blocking — L5 |
| `aarch64-unknown-linux-gnu` | QEMU (ubuntu-latest) | ✅ Blocking — L5 |
| `riscv64gc-unknown-linux-gnu` | QEMU (ubuntu-latest) | ✅ Blocking — L5 |

---

## Tier A+ — Strongly recommended; advisory until promotion

These targets address libc/ABI, pointer-width, and endianness assumptions.  
They are advisory (`continue-on-error: true`) initially.  
Promotion rule: promote to Tier A blocking only after repeated clean scheduled runs with reproducible state-root evidence (L3 minimum, L5 preferred).

| Target triple | Advisory evidence goal | Primary assumption tested | Notes |
|---------------|----------------------|--------------------------|-------|
| `x86_64-unknown-linux-musl` | L3 replay | libc / static-link assumptions | |
| `aarch64-unknown-linux-musl` | L3 replay | libc / static-link assumptions | |
| `i686-unknown-linux-gnu` | L3 replay | 32-bit pointer-width assumptions | `usize = u32`; Domain A explicitly forbids `usize` in consensus state — this verifies the prohibition holds |
| `s390x-unknown-linux-gnu` | L3 replay | Big-endian byte-order assumptions | Critical for wire-format and encoding modules |
| `loongarch64-unknown-linux-gnu` | L1 compile-only initially | China/sovereign hardware posture | QEMU support may vary per runner; promote to L3 when QEMU harness is stable |
| `armv7-unknown-linux-gnueabihf` | L1 compile-only initially | Embedded/edge 32-bit ARM posture | Needs QEMU arm harness; promote to L3 when harness is stable |

---

## Tier B — Hosted OS advisory portability

These targets cover hosted operating systems where Domain A compiles and ideally replays deterministically.  
All advisory (`continue-on-error: true`). No support claim until L3 evidence exists.

| Target triple | Runner | Advisory evidence goal | Notes |
|---------------|--------|----------------------|-------|
| `x86_64-pc-windows-msvc` | `windows-latest` | L3 replay | MSVC toolchain; PE binary format |
| `x86_64-apple-darwin` | `macos-latest` | L3 replay | Mach-O; macOS system libraries |
| `aarch64-apple-darwin` | `macos-latest` (if runner available) | L1 compile-only initially | GitHub-hosted M-series runners are available but subject to quota; best-effort |
| `x86_64-unknown-freebsd` | Cross / QEMU | L2 unit tests | BSD libc; different system call ABI |
| `wasm32-wasip1` | Wasmtime | L3 replay | WASI system interface; WebAssembly stack semantics |
| `wasm32-unknown-unknown` | Embedded / browser | L1 compile-only | No system interface; verifies `no_std` boundary |

---

## Tier C — Embedded / RTOS portability profiles

These profiles target embedded and real-time operating system environments.

**Domain A rule**: Domain A compiles as a bare `no_std` replay kernel under any of these RTOS profiles. RTOS task scheduling, timers, queues, storage, networking, and transport are Domain B PAL concerns. They must not enter Domain A. See [`rtos_portability_plan.md`](rtos_portability_plan.md) for per-RTOS adapter requirements.

Evidence milestone for Tier C entry: L1 compile under a representative bare-metal or RTOS-hosted `no_std` target.

| RTOS / profile | Family | Domain A status | Domain B PAL plan |
|----------------|--------|-----------------|-------------------|
| ITRON / μITRON | Japanese embedded standard | Planned (no_std compile target) | RTOS PAL adapter — task, timer, message queue abstraction |
| μT-Kernel / T-Kernel | TRON family (μITRON successor) | Planned (no_std compile target) | RTOS PAL adapter |
| FreeRTOS | Amazon / open-source RTOS | Planned (no_std compile target) | RTOS PAL adapter |
| Zephyr | Linux Foundation embedded RTOS | Planned (no_std compile target) | RTOS PAL adapter |
| RTEMS | Hard real-time, aerospace/defence | Planned (no_std compile target) | RTOS PAL adapter |
| ThreadX / Azure RTOS | Microsoft embedded RTOS | Planned | RTOS PAL adapter |
| VxWorks | Wind River, hard real-time | Planned | Commercial RTOS PAL adapter |
| QNX | BlackBerry, POSIX RTOS | Planned | POSIX-compatible PAL adapter |
| INTEGRITY | Green Hills, high-assurance | Planned | High-assurance PAL adapter |
| seL4 / eChronos | Formally verified microkernel | Planned | Microkernel PAL adapter |
| AUTOSAR Classic / Adaptive | Automotive RTOS | Planned | Automotive PAL adapter |

---

## Tier D — Accelerator / hardware evidence profiles

These profiles enable Domain B workloads: ZK proving, proof aggregation, batch verification, evidence generation, indexing, and operator tooling.

**Domain A rule**: Accelerators and hardware security modules must never become authoritative Domain A consensus execution engines. They must not change Domain A state-root semantics, introduce accelerator scheduling into consensus, or alter replay equivalence. Any accelerator integration belongs in Domain B and passes only validated scalar effects, commitments, or bounded evidence handles into Domain A.

See [`accelerator_profiles.md`](accelerator_profiles.md) for per-accelerator and per-device adapter requirements.

### GPU / compute accelerators

| Accelerator | Vendor | Domain B use case | Domain A rule |
|-------------|--------|-------------------|---------------|
| Moore Threads / MUSA | Moore Threads (China/sovereign) | ZK proving, proof aggregation, batch verification acceleration, evidence generation | Must not alter Domain A state roots or replay semantics |
| NVIDIA CUDA | NVIDIA | ZK proving, batch verification, simulation | As above |
| AMD ROCm | AMD | ZK proving, batch verification | As above |
| Intel oneAPI / Level Zero | Intel | ZK proving, operator tooling | As above |
| Vulkan compute | Khronos / cross-vendor | GPU compute abstraction | As above |
| OpenCL | Khronos / cross-vendor | Legacy GPU compute | As above |
| Apple Metal | Apple | macOS/iOS GPU compute | As above |

### Hardware security / attestation devices

| Device / profile | Standard | Domain B use case | Domain A rule |
|------------------|----------|-------------------|---------------|
| TPM 2.0 | TCG TPM 2.0 | Hardware attestation, key sealing, platform evidence | Attestation output is a Domain B commitment; does not enter Domain A arithmetic |
| HSM / PKCS#11 | OASIS PKCS#11 | Key storage, signing, hardware crypto acceleration | Key material stays in Domain B; signing outputs are commitments |
| Smartcard / JavaCard | ISO 7816 / GlobalPlatform | Secure element, credential storage | As above |
| TEE / TrustZone | Arm TrustZone, OP-TEE | Trusted execution environment, local evidence generation | TEE evidence is a Domain B commitment |
| SGX-style enclave | Intel SGX (reference) | Confidential compute, local evidence | Enclave output is a Domain B commitment |

---

## Promotion policy

A platform moves from advisory to blocking only when:

1. L3 state-root evidence exists (canonical root matches native x86\_64 reference) across at least three independent scheduled CI runs.
2. L4 replay corpus evidence exists (v1.1 and v1.2 vectors match pinned roots).
3. An L5 evidence artifact is captured and committed to `artifacts/evidence/`.
4. A PR amending this table from "advisory" to "blocking" is approved and merged.

No verbal claim of support substitutes for this evidence gate.

---

## Current CI workflow coverage

| Workflow | Scope | Status |
|----------|-------|--------|
| `platform-determinism.yml` | Tier A (x86\_64, aarch64, riscv64gc) | Blocking |
| `platform-determinism-advisory.yml` | Tier A+ (musl, i686, s390x, LoongArch, ARMv7) | Planned — advisory |
| `os-determinism-advisory.yml` | Tier B (Windows, macOS, WASM) | Planned — advisory |
| `embedded-nostd-advisory.yml` | Tier C entry (bare-metal no\_std compile check) | Planned — advisory |

---

## Non-claims

The following statements are **not** authorised claims at this stage:

- QASH supports all target platforms.
- QASH runs on ITRON / FreeRTOS / Zephyr / any RTOS.
- QASH accelerates proving with Moore Threads / MUSA / CUDA / ROCm.
- QASH integrates with HSMs / TPMs / smartcards / TEEs.
- Domain A is verified on any Tier B, C, or D target.

These are planned evidence targets. They may be stated as design intent using bounded language such as:

> QASH Domain A is designed to be platform-agnostic and `no_std` from inception. Support for additional OS, RTOS, accelerator, and hardware profiles is evidence-gated and must not alter Domain A state-root semantics.
