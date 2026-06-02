# Domain A/B Full Boundary Scan

**Commit:** `698887404a4b0f0cf5e5f83d3f6285bc9e4b7f5c`  
**Timestamp:** 2026-06-01T23:36:01Z  
**Status:** ✅ PASS — Domain A boundary is clean

## Scope

Extends `check_domain_a_tripwires.sh` with a full scan of `crates/consensus/src`
for Domain B imports and platform/accelerator/hardware contamination.

## Standard Domain B import patterns (blocking)

| Pattern | Label |
|---------|-------|
| `qash_pal::` | qash_pal:: (Domain B PAL import) |
| `qash_address::` | qash_address:: (Domain B address import) |
| `use std::net` | use std::net (network I/O) |
| `use std::fs` | use std::fs (filesystem I/O) |
| `use std::env` | use std::env (environment access) |
| `std::time::` | std::time:: (wall clock) |
| `SystemTime` | SystemTime (wall clock) |
| `Instant` | Instant (monotonic clock) |
| `OsRng` | OsRng (entropy) |
| `getrandom` | getrandom (entropy) |
| `rand::` | rand:: (nondeterminism) |
| `serde_json` | serde_json (serialization coupling) |
| `log::` | log:: (logging/tracing) |
| `tracing::` | tracing:: (logging/tracing) |
| `tokio::` | tokio:: (async runtime) |
| `async[[:space:]]+fn` | async fn (async function) |
| `\.await` | .await (async suspension point) |

## Platform/accelerator/hardware patterns (blocking)

| Pattern | Label |
|---------|-------|
| `itron::` | itron:: (ITRON RTOS) |
| `[Ff]reertos` | freertos (FreeRTOS) |
| `[Zz]ephyr` | zephyr (Zephyr RTOS) |
| `[Rr]tems` | rtems (RTEMS) |
| `[Vv]xworks` | vxworks (VxWorks) |
| `[Qq]nx` | qnx (QNX) |
| `cuda::` | cuda:: (NVIDIA CUDA) |
| `rocm::` | rocm:: (AMD ROCm) |
| `musa::` | musa:: (Moore Threads MUSA) |
| `opencl::` | opencl:: (OpenCL) |
| `vulkan::` | vulkan:: (Vulkan compute) |
| `metal::` | metal:: (Apple Metal) |
| `onedal::` | onedal:: (Intel oneDAL) |
| `tpm::` | tpm:: (TPM) |
| `pkcs11::` | pkcs11:: (HSM/PKCS#11) |
| `javacard::` | javacard:: (JavaCard) |
| `sgx::` | sgx:: (SGX enclave) |
| `trustzone::` | trustzone:: (TrustZone) |

## Results

✅ **No violations found.** Domain A boundary is clean.

- No Domain B imports in `crates/consensus/src`
- No RTOS, accelerator, or hardware contamination

## Policy reminder

> Domain A is the deterministic `no_std` replay kernel. RTOS, accelerator,
> and hardware profiles belong in Domain B PAL adapters. They must not alter
> Domain A state-root semantics, introduce nondeterminism, or compromise
> replay equivalence across authorised ISAs.

## Verdict

**PASS** — Domain A/B boundary is intact.
