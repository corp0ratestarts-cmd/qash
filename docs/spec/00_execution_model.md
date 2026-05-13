# QASH Execution Model
## `docs/spec/00_execution_model.md` — Protocol Version 1.0

> **Status:** Canonical execution law. All implementation is constrained by this document.
> `01_consensus.md` depends on the semantics defined here.
> Modifying this document requires a new genesis.

---

## Purpose

This document defines the **deterministic execution substrate** on which all consensus
logic runs. It is not about what the protocol computes — that is `01_consensus.md` —
but about the computational laws under which it computes.

Every implementation must satisfy this document before it can claim to implement QASH.

---

## §E0 — Execution Domain Partition (Protocol Law)

All protocol execution is permanently partitioned into two domains.
This partition is not an implementation guideline — it is protocol law.
Any code, data flow, or value that violates the partition boundary makes the
implementation non-conforming regardless of test results.

---

### Domain A — Deterministic Consensus Domain

**Everything subject to replay invariance and formal proof.**

```
Scope:
  - State transition function T(S_t, I_t)
  - V_convergence(S_t) and Φ_safety(S_t)
  - Canonical encoding Encode(S_t)
  - State root computation R_t
  - Input admissibility verification
  - Epoch advancement and halt evaluation
  - All cryptographic operations over consensus data

Properties (enforceable, not aspirational):
  - Replay invariant across all Tier A authorized ISAs
  - Proof-eligible: formal theorems may quantify over Domain A execution
  - No unsafe blocks
  - No entropy ingress from outside the protocol
  - No allocator-order-dependent behavior
  - No nondeterministic iteration order
  - All arithmetic checked; overflow triggers absorbing halt

Unsafe blocks: FORBIDDEN
```

---

### Domain B — PAL / Operational Domain

**Everything excluded from replay invariance scope.**

```
Scope:
  - Wall-clock time and epoch timing
  - Network I/O (send/recv)
  - Hardware attestation
  - OS scheduler interaction
  - Logging, instrumentation, metrics
  - Human operator interfaces
  - Cryptographic acceleration (SIMD, hardware AES, etc.)
  - Zero-copy serialization with platform-specific alignment

Properties:
  - Nondeterministic behavior permitted
  - Hardware-specific optimization permitted
  - unsafe blocks permitted under formal audit
  - Excluded from all replay theorem scopes (TH-1 through TH-8)
  - Must NOT pass nondeterministic values into Domain A

Unsafe blocks: PERMITTED under PAL audit
```

---

### Boundary rules

The following are **unconditional boundary violations**:

```
VIOLATION:  Any value originating in Domain B that influences a Domain A computation
VIOLATION:  Any Domain A function that calls a Domain B function
VIOLATION:  Clock, entropy, or network state used as input to T(S_t, I_t)
VIOLATION:  Attestation results used to alter state transition semantics
```

The following are **permitted boundary crossings**:

```
PERMITTED:  Domain A outputs (state roots, halt signals) passed to Domain B
PERMITTED:  Domain B providing externally-signed inputs I_t that are then
            verified by Domain A admissibility checks before use
PERMITTED:  Domain B invoking Domain A replay verification (read-only)
```

---

**Cross-domain contamination is a protocol violation.**
An implementation may pass all tests and still be non-conforming
if a Domain B value reaches Domain A computation.

---

## §E1 — Integer Arithmetic Law

All arithmetic in the deterministic domain obeys the following laws without exception.

### Primitive types

Only the following integer types are permitted:

```
u8, u16, u32, u64, u128
i8, i16, i32, i64, i128
bool  (encoded as u8: 0x00 = false, 0x01 = true)
```

Types `usize` and `isize` are **forbidden** in the deterministic domain because their
width is platform-dependent.

### Arithmetic semantics

The deterministic domain uses **checked arithmetic with absorbing halt** throughout.
There is no saturating arithmetic in Domain A. The `Σ_i,t` slash accumulator
uses an explicit pre-checked conditional cap — this is state transition semantics,
not arithmetic semantics. See `01_consensus.md §4b`.

| Context | Semantics |
|---------|-----------|
| All Domain A arithmetic | Checked: overflow triggers absorbing halt |
| Lyapunov intermediate products | Checked `i128` |
| Epoch counter | Checked `u64` |
| `Σ_i,t` slash accumulator | Pre-checked cap at `INT_MAX` via conditional, not saturation |
| Encoding byte offsets | Checked `u32` or `u64` |

**No arithmetic in Domain A may be unchecked, wrapping, or saturating.**

### Canonical `Σ_i,t` update rule

The slash accumulator cap is a **control-flow decision**, not arithmetic saturation.
The canonical form is:

```
// Both operands checked before comparison — overflow here triggers absorbing halt
let sum: i128 = (current as i128) + (increment as i128);
if sum > INT_MAX {
    current = INT_MAX;        // control-flow assignment, not saturation
} else {
    current = sum as i64;     // safe: sum ≤ INT_MAX = 2^63 - 1
}
```

The intermediate `sum` is computed as `i128` so the comparison `sum > INT_MAX`
is itself overflow-free. If `increment` is somehow out of the `i64` range
(which admissibility constraints must prevent), the `i128` addition will still
not overflow, but the transition that produced such an increment is inadmissible
and must be rejected before this point.

### Shift operations

All bit shifts must be by constant or explicitly range-checked values.
Shift by `≥ bit-width` is a protocol halt condition.

### Fixed-point arithmetic

All protocol values that represent fractional quantities use the `𝔽_p` representation:

```
scale:     p = 1_000_000
width:     i64 for storage, i128 for intermediate computation
rounding:  floor toward negative infinity
           (for positive quotients: integer division; for negative: subtract 1 if remainder ≠ 0)
```

The rounding rule is canonically defined as:

```
floor_div(a: i128, b: i128) -> i128:
  q = a / b                    // Rust truncating division
  r = a % b
  if r != 0 && (a ^ b) < 0:   // signs differ and there is a remainder
    q - 1
  else:
    q
```

> **Implementation note:** Rust's default `/` operator for signed integers truncates
> toward zero, which differs from `floor_div` when the result is negative with a
> non-zero remainder. QASH requires Euclidean (floor) division throughout Domain A.
> Implementations must use `i128::div_euclid()` and `i128::rem_euclid()`, or
> explicitly apply the correction above. Using Rust's default `/` for signed
> fixed-point division is a protocol violation that will cause replay divergence
> on negative intermediate values.

This function is part of the protocol and must produce identical results on all platforms.

---

## §E2 — Forbidden Operations

The following operations are **unconditionally forbidden** in the deterministic domain.
This list is exhaustive. If an operation is not explicitly permitted, it is forbidden.

### Floating point

```
FORBIDDEN: f32, f64, f128 (if ever stabilized)
FORBIDDEN: any call to a function that internally uses floating-point arithmetic
FORBIDDEN: SIMD intrinsics with floating-point lanes in the D domain
           (integer SIMD lanes are permitted subject to cross-ISA determinism verification)
```

Rationale: IEEE 754 arithmetic may produce platform-divergent NaN representations,
rounding differences under FMA fusion, and is not reproducible across all authorized ISAs.

### Nondeterministic ordering

```
FORBIDDEN: HashMap, HashSet with random seeds (use BTreeMap, BTreeSet)
FORBIDDEN: Any collection whose iteration order depends on allocation address
FORBIDDEN: Parallel iterators over protocol state (rayon, etc.)
```

### Allocator-dependent behavior

```
FORBIDDEN: Heap allocation in the deterministic domain
           (all data structures must be statically bounded)
FORBIDDEN: Structure layout that differs across allocator versions
FORBIDDEN: Drop order that has semantic side-effects in the D domain
```

### Undefined behavior

```
FORBIDDEN: unsafe blocks in Domain A (the deterministic consensus domain)
           unsafe is permitted in Domain B (PAL) under formal audit — see §E0
FORBIDDEN: Any operation with defined-but-platform-dependent behavior
           (e.g. integer overflow before explicit checked arithmetic)
FORBIDDEN: Transmuting types across endian boundaries
```

### Clock and entropy

```
FORBIDDEN: std::time::SystemTime, Instant, or any wall-clock access in D domain
FORBIDDEN: os-provided random bytes as input to state transition
           (protocol entropy_seed is the only admissible entropy source)
```

### Panics and exits

```
FORBIDDEN: panic!() in the deterministic domain (must use absorbing_reset() instead)
FORBIDDEN: unwrap(), expect() on protocol-path code paths
           (use explicit match or if-let with halt on failure)
FORBIDDEN: std::process::exit() outside of Halt::absorbing_reset()
```

---

## §E3 — Memory Model

### Static bounds

No unbounded, allocator-dependent, or nondeterministically-sized allocation
is permitted in the deterministic domain. All data structures must have a
statically-known maximum size derivable from genesis constants at compile time.
Statically-bounded allocators are permitted provided their layout is deterministic
and independent of allocation order.

### Endianness

All multi-byte integer values in the deterministic domain are **little-endian**
in canonical form. This applies to:

- All fields in `Encode(S_t)`
- All hash inputs that include integer values
- All cryptographic domain separation tags that include integers

### Alignment

No code in the deterministic domain may rely on or assume any particular memory
alignment. All reads and writes must be via properly-typed, alignment-safe operations.

---

## §E4 — Hash Primitive Law

All cryptographic hash operations in the deterministic domain use the following
canonical construction unless explicitly specified otherwise.

### Consensus hash suite

State roots are not defined by a single primitive. Each active consensus
primitive hashes the same domain-separated preimage, and the primitive outputs
are folded into one 32-byte state root. The active v1 suite is:

```
SHA3-256(input) → [u8; 32]
SM3-256(input)  → [u8; 32]
```

`SHA3-256` remains the fold/hash used for entropy advancement and
`GENESIS_CONSTANTS.toml` lock hashing, but state-root security depends on the
entire active digest set. A primitive that is merely logged outside this
construction is not consensus-active.

Input must be a single contiguous byte slice. No streaming API is used in the D domain
(streaming APIs permit platform-divergent buffering behavior).

### Domain separation

All hash inputs in the D domain are domain-separated to prevent cross-context collisions.
Domain tags are 4-byte little-endian u32 values prepended to the input.

```
Domain tag assignments:
  0x00000001  STATE_ROOT       — Encode(S_t) → primitive sub-roots
  0x00000002  ENTROPY_ADVANCE  — seed_t → seed_{t+1}
  0x00000003  VALIDATOR_ID     — validator_id generation
  0x00000004  LEAF_HASH        — sparse Merkle leaf
  0x00000005  INTERNAL_HASH    — sparse Merkle internal node
  0x00000006  CONSENSUS_ROOT   — folds active primitive sub-roots
```

Canonical single-primitive form:

```
H_domain(tag: u32, input: &[u8]) → [u8; 32]:
  SHA3-256( tag.to_le_bytes() ∥ input )
```

Consensus state-root form:

```
root_SHA3 = SHA3-256(STATE_ROOT.to_le_bytes() ∥ input)
root_SM3  = SM3-256(STATE_ROOT.to_le_bytes() ∥ input)

H_consensus_domain(STATE_ROOT, input) =
  SHA3-256(
    CONSENSUS_ROOT.to_le_bytes()
    ∥ 2u32.to_le_bytes()
    ∥ SHA3_PRIMITIVE_ID.to_le_bytes() ∥ root_SHA3
    ∥ SM3_PRIMITIVE_ID.to_le_bytes()  ∥ root_SM3
  )
```

Validators must compute every active primitive. Divergence in any primitive
changes the folded root and is consensus-visible.

### Hash cascade

For operations requiring the full hash cascade (see `GENESIS_CONSTANTS.toml`):

```
H_cascade(input: &[u8]) → [u8; 32]:
  SHA3-256( BLAKE3( KangarooTwelve(input) ) )
```

The cascade is used for: obfuscation layer leaf hashing.
It is **not** used for state root computation; state roots use
`H_consensus_domain` above.

---

## §E5 — Absorbing Halt Semantics

An absorbing halt is an **irreversible terminal state** for the current network instance.

### Trigger conditions

The following conditions unconditionally trigger absorbing halt:

```
H1: ΔV_t > ε  (Lyapunov stability violation, see §5 of 01_consensus.md)
H2: i128 overflow in Lyapunov intermediate computation
H3: u64 overflow of epoch counter
H4: Decode(bytes) returns DecodeResult::Invalid on the local state root
H5: Canonical encoding of S_t does not round-trip through Decode(Encode(S_t)) = S_t
H6: halt_flag = true in any admitted S_t
```

### Halt behavior

```
On halt trigger:
  1. Freeze epoch advancement immediately.
     No S_{t+1} is computed or committed.
  2. Preserve the last valid state root R_{t-1}.
     This root is the succession anchor.
  3. Reject all incoming transitions.
     Return DecodeResult::Invalid with reason 0xFF (HALTED) for all future inputs.
  4. Signal the PAL layer via Halt::absorbing_reset().
     On embedded targets: trigger hardware watchdog.
     On hosted targets: std::process::exit(1).
  5. The halt is network-wide if and only if a quorum of validators halt on the same
     epoch. A single-validator halt does not halt the network; it removes that
     validator from the active set.
```

### Succession

A successor network `G'` anchoring to `R_{t-1}` is the only valid recovery path.
There is no in-protocol recovery from a halt. No governance mechanism can override it.

---

## §E6 — ISA Support Policy

### Authorized platforms

The deterministic domain is guaranteed to produce identical results on:

```
Tier A (primary, CI-verified):
  x86_64   with AVX2
  aarch64  with NEON
  riscv64  with V-extension (vector)

Tier B (authorized, verified on release):
  loongarch64
  riscv64  without V-extension (scalar fallback)
  arm      (32-bit, Cortex-M, ITRON targets)

Tier C (authorized, attested via software-hash-Merkle):
  esp32s3
  itron_renesas_rx
  itron_renesas_rl78
```

### Verification requirement

RT-1 (replay invariance) is tested across at minimum all Tier A platforms in CI
on every push. Tier B and C platforms are verified on tagged release builds.

### ISA-specific restrictions

```
x86_64:  No x87 FPU instructions in D domain (use SSE2 integer ops only)
aarch64: No VFP or ASIMD floating-point lanes in D domain
riscv64: No F/D/Q extensions in D domain; Zbb permitted for bit operations
arm32:   Thumb-2 only; no ARM NEON floating-point in D domain
```

---

## §E7 — Compilation Requirements

To satisfy RT-1, builds must be reproducible across:
- Compiler versions (pinned via `rust-toolchain.toml`)
- Build timestamps (suppressed via `SOURCE_DATE_EPOCH=0`)
- Link IDs (suppressed via `--build-id=none`)
- Incremental artifacts (disabled via `CARGO_INCREMENTAL=0`)
- Codegen units (single via `codegen-units=1` in release profile)

Any binary that is not reproducible under these constraints is not a conforming
QASH build, regardless of whether its output passes test vectors.

---

*End of `docs/spec/00_execution_model.md`*
