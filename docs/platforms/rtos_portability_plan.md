# RTOS Portability Plan

**Status:** Pre-genesis planning document.  
**Parent:** [`authorized_platform_matrix.md`](authorized_platform_matrix.md) — Tier C profiles.

---

## Core rule

> **RTOS support means Domain A compiles and replays deterministically under a bounded `no_std` profile, while RTOS tasking, timers, queues, storage, and transport remain Domain B PAL concerns.**

Domain A (`qash-consensus`) is already `#![no_std]` and `#![forbid(unsafe_code)]`. This means it can in principle be compiled for any embedded target that provides the types Domain A uses (fixed-width integers, `bool`, `BTreeMap`). No RTOS runtime primitives — tasks, semaphores, mutexes, message queues, timers, interrupt handlers, file systems, or network stacks — may enter Domain A.

Evidence required before any RTOS is listed as supported: see [`authorized_platform_matrix.md` — Evidence levels](authorized_platform_matrix.md#evidence-levels).

---

## 1. ITRON / μITRON profile

**Standard:** ITRON (Industrial TRON), μITRON 4.0/4.1, T-Kernel 2.0  
**Origin:** Japan; Embedded Technology Research Organization (TRON Project)  
**Typical targets:** SH, RL78, RX, V850, ARM Cortex-M, MIPS

### Domain A constraints

- Domain A compiles as a `no_std` static library.
- No ITRON API calls (`acre_tsk`, `sta_tsk`, `slp_tsk`, `wai_sem`, `snd_mbx`, `rcv_mbx`, etc.) in Domain A.
- No ITRON memory partition (MPF/MBF) types in Domain A.
- No ITRON time (`get_tim`, `dly_tsk`, systime) in Domain A.

### Domain B PAL adapter requirements

| PAL trait | ITRON implementation |
|-----------|---------------------|
| `Time` | `get_tim()` → convert SYSTIM ticks to Domain B epoch counter |
| `Net` | Task-to-task message box (`mbx`) or mailbox; bounded queues only |
| `Attest` | None initially; hardware attestation deferred to Tier D |
| `Halt` | `ext_tsk()` or `ext_ker()` depending on safety profile; must be irreversible |

### Evidence milestone

L1: `cargo check -p qash-consensus --no-default-features --target <itron-compatible-triple>` succeeds.  
L3: State-root replay matches x86_64 reference under ITRON-hosted test harness.

### Non-claims

QASH does not currently have an ITRON PAL adapter. ITRON support is a planned evidence target.

---

## 2. μT-Kernel / T-Kernel profile

**Standard:** T-Kernel 2.0 (μT-Kernel is the subset for microcontrollers)  
**Origin:** TRON Project, successor to μITRON  
**Typical targets:** ARM Cortex-M0/M3/M4, RISC-V RV32

### Domain A constraints

Same as ITRON: no T-Kernel API (`tk_cre_tsk`, `tk_slp_tsk`, `tk_snd_mbf`, `tk_rcv_mbf`, `tk_get_tim`) in Domain A.

### Domain B PAL adapter requirements

Same shape as ITRON PAL adapter. μT-Kernel adds a Subsystem Manager interface (`tk_def_ssy`) that may be used for Domain B lifecycle management — must not feed into Domain A computations.

### Evidence milestone

L1: `cargo check -p qash-consensus --no-default-features --target thumbv7em-none-eabihf` succeeds (representative μT-Kernel target).

### Non-claims

QASH does not currently have a μT-Kernel PAL adapter. μT-Kernel support is a planned evidence target.

---

## 3. FreeRTOS profile

**Standard:** FreeRTOS / Amazon FreeRTOS (AWS IoT)  
**Origin:** Richard Barry / Amazon Web Services  
**Typical targets:** ARM Cortex-M, RISC-V, Xtensa (ESP32), MIPS

### Domain A constraints

No FreeRTOS API (`xTaskCreate`, `vTaskDelay`, `xQueueSend`, `xQueueReceive`, `xSemaphoreTake`, `xSemaphoreGive`, `pvPortMalloc`, `vPortFree`) in Domain A.

FreeRTOS heap allocators (`heap_1` through `heap_5`) must not be used by Domain A. Domain A is no-alloc by design.

### Domain B PAL adapter requirements

| PAL trait | FreeRTOS implementation |
|-----------|------------------------|
| `Time` | `xTaskGetTickCount()` → Domain B epoch counter; never enters Domain A |
| `Net` | `xQueueSend` / `xQueueReceive` for inter-task message passing |
| `Attest` | None initially; deferred to Tier D hardware integration |
| `Halt` | `vTaskDelete(NULL)` or `configASSERT(0)` depending on safety profile |

### Rust integration path

`freertos-rust` or `freertos_rs` crates provide Rust bindings. All bindings stay in Domain B (`qash-pal`). Domain A has no dependency on these crates.

### Evidence milestone

L1: `cargo check -p qash-consensus --no-default-features --target thumbv7em-none-eabihf` succeeds.

### Non-claims

QASH does not currently have a FreeRTOS PAL adapter. FreeRTOS support is a planned evidence target.

---

## 4. Zephyr profile

**Standard:** Zephyr RTOS (Linux Foundation)  
**Typical targets:** ARM Cortex-M/A, RISC-V, x86, ARC, Xtensa

### Domain A constraints

No Zephyr kernel API (`k_thread_create`, `k_sleep`, `k_msgq_put`, `k_msgq_get`, `k_sem_take`, `k_sem_give`, `k_mutex_lock`, `k_mutex_unlock`, `k_uptime_get`, `sys_rand32_get`) in Domain A.

No Zephyr logging (`LOG_DBG`, `LOG_INF`) or shell APIs in Domain A.

### Domain B PAL adapter requirements

Zephyr provides a native POSIX simulation target (`native_posix`) which allows Domain B development and testing on Linux before hardware deployment. Domain A is compiled as a static library linked into the Zephyr application image.

| PAL trait | Zephyr implementation |
|-----------|----------------------|
| `Time` | `k_uptime_get()` → Domain B epoch counter |
| `Net` | `k_msgq_put` / `k_msgq_get` for bounded queues |
| `Attest` | None initially; deferred to Tier D |
| `Halt` | `k_panic()` or `k_fatal_halt()` |

### Evidence milestone

L1: `cargo check -p qash-consensus --no-default-features --target thumbv8m.main-none-eabihf` succeeds (representative Zephyr Cortex-M33 target).

### Non-claims

QASH does not currently have a Zephyr PAL adapter. Zephyr support is a planned evidence target.

---

## 5. RTEMS profile

**Standard:** RTEMS (Real-Time Executive for Multiprocessor Systems)  
**Typical targets:** SPARC (LEON), PowerPC, ARM, RISC-V, x86  
**Application domain:** Aerospace, defence, scientific instruments

### Domain A constraints

No RTEMS API (`rtems_task_create`, `rtems_task_start`, `rtems_task_delay`, `rtems_message_queue_send`, `rtems_clock_get_uptime`, `rtems_fatal`) in Domain A.

### Domain B PAL adapter requirements

RTEMS provides POSIX compatibility; Domain B PAL adapter can use POSIX-layer APIs (`clock_gettime`, `pthread_create`, `mq_send`, `mq_receive`) where the POSIX layer is available.

### Evidence milestone

L1: Compile Domain A as a `no_std` RTEMS static library via the RTEMS Source Builder toolchain.

### Non-claims

QASH does not currently have an RTEMS PAL adapter. RTEMS support is a planned evidence target.

---

## 6. High-assurance RTOS profiles (VxWorks / QNX / INTEGRITY)

These RTOSes are used in safety-critical and defence contexts. All three support static partitioning and certified kernel variants.

| RTOS | Vendor | Certification | Typical domain |
|------|--------|--------------|----------------|
| VxWorks | Wind River | DO-178C, IEC 61508, ISO 26262 | Aerospace, defence, automotive |
| QNX Neutrino | BlackBerry | IEC 61508, ISO 26262, POSIX | Automotive, medical, defence |
| INTEGRITY | Green Hills | DO-178C Level A, Common Criteria EAL6+ | Avionics, defence |

### Domain A constraints

Same rule applies: no proprietary RTOS API, no shared memory, no IPC primitives in Domain A. Domain A is a `no_std` static archive.

### Domain B PAL adapter requirements

All three support POSIX Process Model or proprietary IPC. Domain B PAL adapter uses the POSIX layer or native IPC exclusively. Deterministic task scheduling is a Domain B concern and must not alter Domain A state-root semantics.

### Evidence milestone

L1: Cross-compile Domain A as a `no_std` static archive for the target toolchain (e.g., VxWorks Diab compiler, QNX `qcc`, INTEGRITY `cxarm`).

### Non-claims

QASH does not currently have PAL adapters for VxWorks, QNX, or INTEGRITY. These are planned evidence targets.

---

## 7. seL4 / formally verified RTOS profiles

**Standard:** seL4 microkernel (CSIRO Data61 / Neutrality Foundation)  
**Typical targets:** ARM Cortex-A, RISC-V, x86_64  
**Application domain:** High-assurance, avionics (DARPA HACMS), defence

### Domain A constraints

seL4 provides a capability-based microkernel. Domain A is compiled as a `no_std` component. No seL4 IPC (`seL4_Call`, `seL4_Send`, `seL4_Recv`, `seL4_ReplyRecv`) or capability management (`seL4_CNode_*`) in Domain A.

### Domain B PAL adapter requirements

Domain B PAL adapter uses seL4 IPC endpoints for message passing. The seL4 formal proof covers kernel correctness; Domain A replay correctness is a separate and orthogonal guarantee.

### Evidence milestone

L1: `cargo check -p qash-consensus --no-default-features --target aarch64-unknown-none` succeeds (representative seL4 target).

### Non-claims

QASH does not currently have a seL4 PAL adapter. seL4 support is a planned evidence target.

---

## 8. AUTOSAR profile

**Standard:** AUTOSAR Classic Platform / AUTOSAR Adaptive Platform  
**Typical targets:** Classic: TriCore, RH850, ARM Cortex-M; Adaptive: ARM Cortex-A (POSIX)

### Domain A constraints

No AUTOSAR RTE (Runtime Environment) API, no COM stack, no DCM/DEM calls in Domain A. Domain A is a `no_std` static library linked into the AUTOSAR SWC (Software Component).

### Domain B PAL adapter requirements

| Platform | PAL implementation |
|----------|-------------------|
| Classic | SWC wrapper calls Domain A; Domain B uses Os API for timing and communication |
| Adaptive | `ara::exec` manages Domain B lifecycle; Domain A linked as a Functional Cluster library |

### Evidence milestone

L1: Cross-compile Domain A for an AUTOSAR Classic target triple (e.g., `tricore-htc-eabi`).

### Non-claims

QASH does not currently have an AUTOSAR PAL adapter. AUTOSAR support is a planned evidence target.

---

## 9. `no_std` Domain A requirements (summary)

For any RTOS target, Domain A must satisfy:

| Requirement | Status |
|-------------|--------|
| `#![no_std]` | ✅ Already enforced |
| `#![forbid(unsafe_code)]` | ✅ Already enforced |
| No heap allocation | ✅ No-alloc design |
| No OS API calls | ✅ Enforced by Domain A tripwires |
| Fixed-width arithmetic only | ✅ Domain A arithmetic rules |
| Overflow → absorbing halt | ✅ `Halt::absorbing_reset()` |
| Deterministic iteration (`BTreeMap`) | ✅ Clippy disallowed-types |
| No wall-clock / entropy ingress | ✅ Domain A tripwires |
| Replay-invariant across ISAs | ✅ TH-7 CI (Tier A) |

---

## 10. RTOS PAL adapter requirements (summary)

Any RTOS PAL adapter added to `qash-pal` must:

1. Implement the `Time`, `Net`, `Attest`, and `Halt` traits defined in `qash-pal`.
2. Never pass RTOS time values directly into Domain A computations — only validated scalar effects and commitments cross the Domain A/B boundary.
3. Use the RTOS API exclusively within `qash-pal`; no RTOS API visible from `qash-consensus`.
4. Treat `Halt::absorbing_reset()` as the irreversible error path: must not restart Domain A computation after an absorbing halt.
5. Compile with `no_std` for the target RTOS environment.

---

## 11. Replay-vector evidence requirements

Before any RTOS target is promoted beyond L1:

- A replay vector test harness must be ported that runs `v1_1_corpus_matches_pinned` and `v1_2_sharded_corpus_matches_pinned` under the RTOS environment.
- The canonical state root must be captured and compared against the native x86_64 reference root.
- Results must be committed as an L5 evidence artifact to `artifacts/evidence/`.

---

## 12. Non-claims boundary

The following statements are **not** authorised claims at this stage:

- QASH supports ITRON / μT-Kernel / FreeRTOS / Zephyr / RTEMS / VxWorks / QNX / INTEGRITY / seL4 / AUTOSAR.
- QASH is safety-certified for any RTOS environment.
- QASH satisfies DO-178C, IEC 61508, ISO 26262, or any other safety standard on any RTOS.
- Domain A has been verified on any RTOS target.

Authorised design-intent language:

> QASH Domain A is designed to be `no_std` and platform-agnostic from inception, enabling future RTOS portability. RTOS tasking, timers, queues, storage, and transport are Domain B PAL concerns. RTOS support claims are evidence-gated and require L3 replay evidence at minimum.
