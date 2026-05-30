# QASH Astronomical Hash Cascade
## `docs/spec/07_hash_cascade.md` — Protocol Version 1.1 (QASH-CASCADE-7)

> **Status:** Canonical specification for `H_cascade` as a post-genesis/v1.1 cascade path. Implements DD-7.
> All cascade implementations must produce identical output to this spec on all Tier A ISAs.
> Captured in `00_execution_model.md §E4` (normative reference).
> **v1.0 state-root note:** Domain A state roots at genesis use `H_domain(STATE_ROOT, ...)` / SHA3-256. They do not use `H_cascade`, a truncation of `H_cascade`, or any cascade commitment rule.

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

Seven primitives are applied in parallel, one per national cryptographic standard,
providing geopolitical multi-polar sovereignty. Each receives a domain-separated
input: `tag ∥ input` where `tag` is the L1 domain separator string encoded
as UTF-8 bytes (no null terminator, no length prefix).

All L1 primitives produce **64 bytes** (512-bit output), ensuring uniform
256-bit quantum collision resistance (Grover's algorithm halves the effective
collision security to 256 bits for 512-bit outputs, vs. 128 bits for 256-bit outputs).

```
L1_sep  = "QASH:CASCADE:L1:PARALLEL"   (UTF-8, 24 bytes)

h1[0] = SHA3-512      ( L1_sep ∥ input )              // [u8; 64]   FIPS 202, USA/NATO
h1[1] = BLAKE3-XOF    ( L1_sep ∥ input, out=64 )      // [u8; 64]   Open, Decentralized
h1[2] = K12-XOF       ( L1_sep ∥ input, out=64 )      // [u8; 64]   KangarooTwelve, EU Academic
h1[3] = SM3×2         ( L1_sep ∥ input )              // [u8; 64]   GB/T 32905-2016, China/SEA (*)
h1[4] = Streebog-512  ( L1_sep ∥ input )              // [u8; 64]   GOST R 34.11-2012, Russia/EAEU
h1[5] = Kupyna-512    ( L1_sep ∥ input )              // [u8; 64]   DSTU 7564:2014, Ukraine
h1[6] = LSH-512       ( L1_sep ∥ input )              // [u8; 64]   KS X 3262, South Korea

parallel = h1[0] ∥ h1[1] ∥ h1[2] ∥ h1[3] ∥ h1[4] ∥ h1[5] ∥ h1[6]  // [u8; 448]
```

(*) **SM3 double-width construction:** SM3 (GB/T 32905-2016) is natively a 256-bit hash
with no official 512-bit variant. To maintain uniform 64-byte L1 outputs, two independent
SM3 invocations with distinct 1-byte domain prefixes are concatenated:

```
h1[3][0..32]  = SM3( 0x01 ∥ L1_sep ∥ input )
h1[3][32..64] = SM3( 0x02 ∥ L1_sep ∥ input )
```

This construction is provably secure under SM3 collision resistance and satisfies
OSCCA compliance (the SM3 compression function is used exclusively).

All other primitives use their native 512-bit output mode (SHA3-512, Streebog-512,
Kupyna-512) or their XOF interface squeezed to exactly 64 bytes (BLAKE3, KangarooTwelve,
LSH-512). Implementations must not truncate or extend outputs beyond 64 bytes per slot.

### Layer 2 — Binding

```
L2_sep = "QASH:CASCADE:L2:BIND"   (UTF-8, 20 bytes)

L2 = SHA3-512( L2_sep ∥ parallel )   // [u8; 64]
```

SHA3-512 is the binding primitive (`binding_primitive = "SHA3-512"` in genesis).
The binding layer compresses the 448-byte L1 entropy pool into a single 64-byte
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

All seven L1 primitives and SHA3-512 must produce bitwise-identical output on all
Tier A ISAs. Hardware acceleration is permitted in Domain B only; the Domain A
cascade path must use reference implementations verified by the cross-ISA test suite.

### Parallelism

L1 computation across the seven primitives may be parallelized in the PAL layer
(Domain B), provided the concatenation order is fixed: SHA3-512, BLAKE3-XOF,
K12-XOF, SM3-double-width, Streebog-512, Kupyna-512, LSH-512 (array indices 0–6).
The Domain A caller receives the concatenated 448-byte `parallel` slice; it does
not observe the parallelism.

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

For v1.0 genesis, state root computation **never** calls `H_cascade`. Cascade health commitment **never** replaces the state root computation. The two functions are orthogonal. Any future cascade-backed state-root migration must define a new commitment/truncation rule, update `compute_state_root`, and replace the genesis KAT vectors in a separate post-genesis ADR.

---

## §4 — Cascade Blinding (Context-Keyed Variant)

To prevent preimage grinding of deterministic cascade inputs and to bind
epoch-local operations to their specific epoch context, the cascade supports a
**context key** that is mixed into the L2 binding layer.

### Formal definition

```
H_cascade_keyed(context_key: &[u8], input: &[u8]) → [u8; 64]
```

The keyed variant is identical to `H_cascade` except at Layer 2:

```
L2 = SHA3-512( L2_sep ∥ context_key ∥ parallel )
```

The unkeyed form is the degenerate case:

```
H_cascade(input) ≡ H_cascade_keyed([], input)
```

`context_key` is always a protocol-level constant or deterministically derived
value — never a secret or nonce sampled from a random source. This preserves
Domain A replay invariance (`00_execution_model.md §E2`, axiom A5 of
`02_transition_axioms.md`).

### Blinding in cascade proofs

When generating a cascade proof for obfuscation leaf `i` at epoch `t`, the
context key is the epoch entropy seed:

```
leaf_hash_i_t = H_cascade_keyed( seed_t, validator_id_i ∥ epoch_t_le8 )
```

where `seed_t = S_t.entropy_seed` (`01_consensus.md §1`) and `epoch_t_le8` is
the 8-byte little-endian encoding of the epoch counter.

This binds each leaf hash to its epoch, preventing cross-epoch proof replay.
The leaf index used in the sparse Merkle tree is the first 48 bytes of
`leaf_hash_i_t`, consistent with `obfuscation.leaf_index_bytes = 48`
(`GENESIS_CONSTANTS.toml [obfuscation]`).

### Genesis hash blinding nonce

The genesis hash computation uses a **genesis blind nonce** pinned in
`GENESIS_CONSTANTS.toml [meta].genesis_blind_nonce`. The nonce is included in
the canonical TOML bytes (it is **not** stripped before hashing, unlike
`genesis_hash` and `lock_algorithm`). Its value is fixed at genesis lock and
cannot be changed without redefining the network.

```
genesis_hash = hex( H_cascade_keyed( [], canonical_genesis_bytes_with_nonce ) )
```

where `canonical_genesis_bytes_with_nonce` is the TOML file with only
`genesis_hash` and `lock_algorithm` values blanked (the nonce line is kept
verbatim).

---

## §5 — Hierarchical Deterministic Cascade Derivation

To support epoch-keyed cascade roots for cascade health proofs and clone-chunk
verification, an HD derivation function is defined:

```
H_cascade_derive(parent_root: [u8; 64], epoch: u64, seed: [u8; 32]) → [u8; 64]
```

Implementation:

```
derive_input = epoch_le8 ∥ seed        // 8 + 32 = 40 bytes
H_cascade_derive(parent_root, epoch, seed) = H_cascade_keyed(parent_root, derive_input)
```

where `epoch_le8` is the 8-byte little-endian encoding of `epoch`.

### Epoch cascade root chain

```
cascade_root_0 = H_cascade( canonical_genesis_bytes )    // genesis root (§3)
cascade_root_t = H_cascade_derive( cascade_root_{t-1}, t, seed_t )
```

`cascade_root_t` is committed in the epoch state and used to anchor cascade
proofs in epoch `t`. Validators must supply cascade proofs against
`cascade_root_t`; proofs against a stale root are rejected (counted in `CH_t`).

### Security properties

- **Forward security**: given `cascade_root_t` and `seed_t`, an observer cannot
  compute `cascade_root_{t-1}` without inverting SHA3-512 (7 layers deep).
- **Epoch binding**: `cascade_root_t` depends on every prior epoch seed, so
  it cannot be forged without controlling the entropy advance chain
  (`H_domain(ENTROPY_ADVANCE, ...)` per `01_consensus.md §1`).
- **Replay invariance**: all inputs are deterministic; no nondeterministic values
  enter the computation (axiom A1 of `02_transition_axioms.md`).

---

## §6 — Implementation Requirements

All implementations of `H_cascade`, `H_cascade_keyed`, and `H_cascade_derive`
**must** use pure-Rust (or safe-Rust) implementations of all seven L1 primitives
and SHA3-512. C FFI, assembly backends, or hardware intrinsic paths are
forbidden in the Domain A cascade path, as they may produce ISA-dependent output
under certain edge cases and cannot be verified by the Coq proof suite.

Concretely for the reference implementation:

| Primitive | Crate | Required feature | Output mode |
|-----------|-------|-----------------|-------------|
| SHA3-512 | `sha3 = "0.10"` | `default-features = false` | Fixed 64-byte digest |
| BLAKE3 | `blake3 = "1.5.5"` | `default-features = false, features = ["pure"]` | XOF `finalize_xof().fill(&mut [0u8;64])` |
| KangarooTwelve | `tiny-keccak = "2"` | `features = ["k12"]` | XOF `finalize(h, &mut [0u8;64])` |
| SM3 | `sm3 = "0.4"` | `default-features = false` | Double-width (2 × 32 bytes, prefixes 0x01/0x02) |
| Streebog-512 | `streebog = "0.10"` | `default-features = false` | Fixed 64-byte digest |
| Kupyna-512 | `kupyna = "0.1.0"` | `default-features = false` | Fixed 64-byte digest |
| LSH-512 | (in-crate `lsh512.rs`) | — | `lsh512_parts(sep, input)` → 64 bytes |

The `pure` feature on `blake3` disables all assembly and C backends, ensuring
the RustCrypto-compatible Rust implementation is used. This requirement is
normative; CI must reject builds where `cc` is invoked transitively from any
cascade-related crate.

---

*End of `docs/spec/07_hash_cascade.md`*
