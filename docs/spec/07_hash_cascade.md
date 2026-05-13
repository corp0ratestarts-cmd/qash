# QASH Astronomical Hash Cascade
## `docs/spec/07_hash_cascade.md` — Protocol Version 1.1

> **Status:** Canonical specification for `H_cascade`. Implements DD-7.
> All cascade implementations must produce identical output to this spec on all Tier A ISAs.
> Captured in `00_execution_model.md §E4` (normative reference).

---

## Purpose

The astronomical hash cascade (`H_cascade`) provides a depth-7, multi-primitive
hash construction used for:

1. Obfuscation layer leaf hashing (`cascade_derived_injective` mode)
2. Clone-chunk verification (`cascade_bound` mode in clone protocol v1.2)
3. Cascade health (CH) commitment included in `V_convergence` (§4c of `01_consensus.md`)

`H_cascade` is **not** used for state root computation, which uses `H_domain`
(SHA3-256, defined in `00_execution_model.md §E4`).

---

## Formal Definition

```
H_cascade(input: &[u8]) → [u8; 64]
```

### Layer 1 — Parallel primitive application

Five primitives are applied in parallel. Each receives a domain-separated
input: `tag ∥ input` where `tag` is the L1 domain separator string encoded
as UTF-8 bytes (no null terminator, no length prefix).

```
L1_sep  = "QASH:CASCADE:L1:PARALLEL"   (UTF-8, 24 bytes)

h1[0] = SHA3-256 ( L1_sep ∥ input )    // [u8; 32]
h1[1] = BLAKE3   ( L1_sep ∥ input )    // [u8; 32]
h1[2] = K12      ( L1_sep ∥ input )    // [u8; 32]   (KangarooTwelve, 32-byte output)
h1[3] = SM3      ( L1_sep ∥ input )    // [u8; 32]
h1[4] = Streebog ( L1_sep ∥ input )    // [u8; 32]   (GOST R 34.11-2012, 256-bit output)

parallel = h1[0] ∥ h1[1] ∥ h1[2] ∥ h1[3] ∥ h1[4]  // [u8; 160]
```

Primitive output sizes are fixed at 32 bytes each. Implementations must use
the 256-bit output variant of each primitive. Any variable-output primitive
must be configured for exactly 32 bytes of output.

### Layer 2 — Binding

```
L2_sep = "QASH:CASCADE:L2:BIND"   (UTF-8, 20 bytes)

L2 = SHA3-512( L2_sep ∥ parallel )   // [u8; 64]
```

SHA3-512 is the binding primitive (`binding_primitive = "SHA3-512"` in genesis).
The binding layer ensures all five L1 outputs are committed into a single 64-byte
value before recursive expansion.

### Layers 3–6 — Recursive expansion

For n ∈ {3, 4, 5, 6}:

```
L{n}_sep = "QASH:CASCADE:L{n}:EXPAND"   (UTF-8, e.g. "QASH:CASCADE:L3:EXPAND" = 22 bytes)

L{n} = SHA3-512( L{n}_sep ∥ L{n-1} )   // [u8; 64]
```

Each layer takes the 64-byte output of the previous layer as input.

### Layer 7 — Finalization

```
L7_sep = "QASH:CASCADE:L7:FINALIZE"   (UTF-8, 24 bytes)

L7 = SHA3-512( L7_sep ∥ L6 )   // [u8; 64]

H_cascade(input) = L7
```

---

## Domain Separators

The seven domain separator strings (from `GENESIS_CONSTANTS.toml`
`[crypto.cascade].domain_separators`) are:

| Layer | Separator string | Byte length |
|-------|-----------------|-------------|
| L1 | `QASH:CASCADE:L1:PARALLEL` | 24 |
| L2 | `QASH:CASCADE:L2:BIND` | 20 |
| L3 | `QASH:CASCADE:L3:EXPAND` | 22 |
| L4 | `QASH:CASCADE:L4:EXPAND` | 22 |
| L5 | `QASH:CASCADE:L5:EXPAND` | 22 |
| L6 | `QASH:CASCADE:L6:EXPAND` | 22 |
| L7 | `QASH:CASCADE:L7:FINALIZE` | 24 |

These strings are fixed at genesis. Their byte lengths are not encoded in the
hash input — the strings themselves are the separators. Length variability is
not a concern because the strings are protocol constants.

---

## Cascade Proof Format

When `cascade_proof_inclusion = "sparse_merkle"`, each cascade output is
committed into the block's cascade sparse-Merkle tree. An inclusion proof
for a cascade computation is:

```
CascadeProof = (
  leaf_index:  [u8; 48],    // epoch-relative leaf index (from obfuscation.leaf_index_bytes)
  l7_output:   [u8; 64],    // H_cascade output
  merkle_path: [[u8; 32]; sparse_merkle_depth],  // depth = 384 inner nodes
)
```

Proof verification: recompute the Merkle root from `leaf_index`, `l7_output`,
and `merkle_path` using `H_domain(LEAF_HASH, ...)` / `H_domain(INTERNAL_HASH, ...)`
(tags 0x00000004 / 0x00000005 from `00_execution_model.md §E4`).

---

## Cascade Health Factor Derivation

`CH_t` (§4c of `01_consensus.md`) is derived from cascade proof rejections
in `I_t`. A cascade proof is **rejected** if:

1. The recomputed Merkle root does not match the block's cascade tree root, or
2. The `l7_output` field does not equal `H_cascade(original_input)` for the
   associated obfuscation leaf or clone chunk.

```
cascade_fail_count_t = |{ proofs in I_t that are rejected }|
CH_t = cascade_fail_count_t × p / max_queries_per_epoch
```

`CH_t` is a Domain A value computed entirely from admitted, signature-verified
inputs. No Domain B values influence it.

---

## Implementation Notes

### Cross-ISA determinism

All five L1 primitives and SHA3-512 must produce bitwise-identical output on all
Tier A ISAs. Hardware acceleration is permitted in Domain B only; the Domain A
cascade path must use reference implementations verified by the cross-ISA test suite.

### Parallelism

L1 computation across the five primitives may be parallelized in the PAL layer
(Domain B), provided the concatenation order is fixed: SHA3-256, BLAKE3, K12, SM3,
Streebog (array indices 0–4). The Domain A caller receives the concatenated
`parallel` slice; it does not observe the parallelism.

### No streaming

All hash inputs are single contiguous slices. No streaming API is used (streaming
permits platform-divergent buffering). This is the same constraint as `H_domain`.

---

## Relationship to DD-3 (SHA3-256 as canonical consensus hash)

`H_cascade` and `H_domain` are **distinct functions with distinct purposes**:

| Function | Purpose | Output size | Domain |
|----------|---------|-------------|--------|
| `H_domain` | State root, entropy seed, Merkle nodes | `[u8; 32]` | Domain A — consensus |
| `H_cascade` | Obfuscation leaf, clone chunk, CH | `[u8; 64]` | Domain A — cascade path |

State root computation **never** calls `H_cascade`. Cascade health commitment
**never** replaces the state root computation. The two functions are orthogonal.

---

*End of `docs/spec/07_hash_cascade.md`*
