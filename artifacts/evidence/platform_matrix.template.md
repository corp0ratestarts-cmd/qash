# Platform Evidence Matrix

<!-- This file is a template. The authoritative generated copy is produced by
     scripts/build_platform_evidence_matrix.sh and committed to
     artifacts/evidence/platform_matrix.md on every full-audit run. -->

**Commit:** `{{COMMIT_SHA}}`  
**Timestamp:** `{{TIMESTAMP}}`  
**Reference state root:** `{{REFERENCE_ROOT}}`

---

## Claim boundary reminder

> Current TH-7 CI covers Linux x86_64/aarch64/riscv64gc only. Additional
> CPU/ISA, OS, RTOS, accelerator, and hardware profiles are planned evidence
> targets. No broader support claim is allowed until compile, replay, and
> evidence artifacts exist for that tier.

---

## Evidence levels

| Level | Name | Criterion |
|-------|------|-----------|
| L1 | Compile | `cargo check` / `cargo build` succeeds for the target |
| L2 | Test | Unit tests pass under cross/QEMU |
| L3 | Replay | Canonical state root matches native x86_64 reference |
| L4 | Corpus | v1.1 and v1.2 replay corpus matches |
| L5 | Artifact | Evidence artifact captured, committed, and indexed here |

---

## Tier A — genesis-blocking (must be L3+ to merge to main)

| Target | Evidence level | State root match | Notes |
|--------|---------------|-----------------|-------|
| `x86_64-unknown-linux-gnu` | L5 | ✅ Reference | Enforced by `platform-determinism.yml` |
| `aarch64-unknown-linux-gnu` | L5 | ✅ Match | Enforced by `platform-determinism.yml` |
| `riscv64gc-unknown-linux-gnu` | L5 | ✅ Match | Enforced by `platform-determinism.yml` |

---

## Tier A+ — advisory ISA (goal: L3+ before promotion to Tier A)

| Target | Evidence level | State root match | Notes |
|--------|---------------|-----------------|-------|
| `x86_64-unknown-linux-musl` | — | — | Planned advisory |
| `aarch64-unknown-linux-musl` | — | — | Planned advisory |
| `i686-unknown-linux-gnu` | — | — | 32-bit; pointer-width assumptions |
| `s390x-unknown-linux-gnu` | — | — | Big-endian assumptions |
| `loongarch64-unknown-linux-gnu` | — | — | Sovereign/China hardware posture |
| `armv7-unknown-linux-gnueabihf` | — | — | QEMU arm harness needed |

---

## Tier B — hosted OS advisory (goal: L2+ evidence; L3+ before claims)

| Target | Evidence level | State root match | Notes |
|--------|---------------|-----------------|-------|
| `x86_64-pc-windows-msvc` | — | — | Planned advisory |
| `x86_64-apple-darwin` | — | — | Planned advisory |
| `aarch64-apple-darwin` | — | — | Best-effort; runner availability varies |
| `x86_64-unknown-freebsd` | — | — | Planned advisory |
| `wasm32-wasip1` | — | — | Planned advisory |
| `wasm32-unknown-unknown` | — | — | Planned advisory |

---

## Tier C — embedded / RTOS profiles (Domain A compile + replay; RTOS APIs remain Domain B)

| Profile | Target triple | Evidence level | Notes |
|---------|---------------|---------------|-------|
| ITRON / μITRON / μT-Kernel / T-Kernel | `thumbv7em-none-eabihf` | — | L1 compile goal |
| FreeRTOS | `thumbv7em-none-eabihf` | — | L1 compile goal |
| Zephyr | `thumbv8m.main-none-eabihf` | — | L1 compile goal |
| RTEMS | `riscv32imac-unknown-none-elf` | — | L1 compile goal |
| VxWorks | TBD | — | Toolchain availability TBD |
| QNX | TBD | — | Toolchain availability TBD |
| INTEGRITY | TBD | — | GHS toolchain required |
| seL4 | `aarch64-unknown-none` | — | L1 compile goal |
| AUTOSAR Classic/Adaptive | `thumbv7em-none-eabihf` | — | L1 compile goal |

---

## Tier D — accelerator / hardware evidence profiles (Domain B only)

| Profile | Permitted Domain B uses | Evidence level | Notes |
|---------|------------------------|---------------|-------|
| Moore Threads / MUSA | ZK proving, aggregation, batch verify, evidence gen | — | Sovereign/China posture |
| NVIDIA CUDA | ZK proving, batch verify, simulation | — | — |
| AMD ROCm | ZK proving, batch verify | — | — |
| Intel oneAPI / Level Zero | ZK proving, operator tooling | — | — |
| Vulkan compute | Cross-vendor ZK proving (SPIR-V) | — | — |
| OpenCL | Legacy/FPGA batch hash | — | — |
| Apple Metal | ZK proving on Apple Silicon | — | — |
| TPM 2.0 | Platform attestation, key sealing | — | — |
| HSM / PKCS#11 | Key storage, signing, batch verify | — | — |
| Smartcard / JavaCard | Credential storage, selective disclosure | — | — |
| TEE / TrustZone / OP-TEE | Local evidence, key isolation | — | — |
| SGX-style enclave | Confidential compute, remote attestation | — | — |

---

## Promotion criteria

To promote a target from advisory to blocking:

1. L3 state-root evidence across ≥ 3 independent scheduled CI runs
2. L4 replay corpus match (v1.1 and v1.2)
3. L5 evidence artifact committed and indexed in this table
4. Approved PR amending `docs/platforms/authorized_platform_matrix.md`
