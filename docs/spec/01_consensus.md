# QASH Consensus Specification
## `docs/spec/01_consensus.md` — Protocol Version 1.0

> **Status:** Canonical root specification. All implementation is constrained by this document.
> Modifying this document requires a new genesis. No exceptions.

---

## Notation

| Symbol | Meaning |
|--------|---------|
| `S_t` | Protocol state vector at epoch `t` |
| `S_t[k]` | Component `k` of state at epoch `t` |
| `T(S_t, I_t)` | State transition function |
| `I_t` | Canonical input set at epoch `t` |
| `V_convergence(S_t)` | Operational Lyapunov candidate — dynamic terms only |
| `Φ_safety(S_t)` | Monotone safety accumulator — slash evidence only |
| `δ_window` | Rolling-window excursion: `V_convergence(S_t) − min(lyapunov_window)`. Not a temporal derivative — measures deviation from the rolling minimum, not V_{t+1} − V_t |
| `ε` | Convergence tolerance (`epsilon_threshold` in genesis) |
| `Φ_max` | Derived safety bound: `N_max × γ × INT_MAX` |
| `Φ_max_safe` | Halt threshold: `Φ_max / 2` |
| `INT_MAX` | `2^63 − 1` (pure arithmetic; maps to `i64::MAX` in Rust) |
| `N_max` | Maximum validator cardinality = `1024` |
| `𝔽_p` | Fixed-point integer field, scale `p = 1_000_000` |
| `i128` | 128-bit signed integer (intermediate arithmetic width) |
| `⊥` | Absorbing halt state (irreversible) |
| `G` | Genesis constants block (immutable) |
| `R_n` | `S_n.state_root` — the stored consensus artifact; admissibility invariant defined in §2 |
| `∥` | Concatenation |
| `H_domain` | Domain-separated hash (defined in `00_execution_model.md §E4`) |

All scalar values are elements of `𝔽_p` unless otherwise stated.
All arithmetic in the deterministic domain uses **checked arithmetic with absorbing halt**
on overflow. There is no saturating arithmetic in the consensus execution path.
There is no floating point anywhere in the consensus execution path.

The single exception is `Σ_i,t` (slash accumulator), which is explicitly capped at
`INT_MAX` via a pre-checked conditional — not via saturation. See §4b.

---

## §0 — Deterministic Execution Constraints

These constraints are **globally binding** across the entire consensus execution path.
Any implementation that violates one is incorrect regardless of test passage.

### Forbidden operations

The following are unconditionally forbidden in any code reachable from `T(S_t, I_t)`,
`V(S_t)`, `Encode(S_t)`, or any function they call:

```
FORBIDDEN:
  f32, f64, or any IEEE 754 floating-point type or operation
  nondeterministic iteration order (HashMap without deterministic seed, BTreeMap is permitted)
  allocator-dependent structure layout or ordering
  clock-derived state mutation (wall time must never enter state)
  host entropy in consensus execution (entropy sources are protocol-derived only)
  signed integer overflow (all arithmetic is checked; overflow triggers absorbing halt)
  architecture-dependent alignment assumptions
  platform-dependent integer widths (use explicit i8/u8/.../i128/u128 only)
  std::process::exit or panic!() outside of absorbing_reset()
  unsafe code blocks
```

### Required properties

Every function in the consensus execution path must be:

```
REQUIRED:
  pure (no side effects on external state)
  total (defined for all inputs in the admissible domain)
  terminating (no unbounded loops without explicit epoch budget)
  deterministic (same input → same output on all authorized platforms)
```

### Fixed-point arithmetic rules

```
scale:              p = 1_000_000  (one unit = 1_000_000 in 𝔽_p)
intermediate width: i128           (no intermediate result may narrow before final assignment)
rounding:           floor toward negative infinity
overflow policy:    absorbing halt  (any result exceeding i128 range triggers ⊥)
```

---

## §0b — Adversarial Scope and Claim Separation

Adversarial assumptions are defined in `04_adversarial_model.md` and are
binding for theorem interpretation.

- Safety claims are unconditional with respect to transport behavior.
- Liveness claims are conditional and must state environmental assumptions.
- Consensus-state determinism claims are Domain-A claims and are independent
  of Domain-B message timing, ordering, and scheduling behavior.

## §1 — State Space

The protocol state at epoch `t` is a tuple:

```
S_t = (
  epoch:         u64,        // monotonically increasing, seeded at genesis
  state_root:    [u8; 32],   // Stores R_t = S_t.state_root (consensus artifact)
                             // Canonical commitment invariant (§2):
                             // R_t = H_consensus_domain(STATE_ROOT, Encode_for_commitment(S_t, prior_root(t)))
  validators:    [V_i; N],   // fixed-size validator array, N defined at genesis
  ledger_root:   [u8; 32],   // root of the sparse Merkle accumulator
  entropy_seed:  [u8; 32],   // forward-secure: seed_{t+1} = SHA3-256(seed_t)
  lyapunov_window: [i64; W], // ring buffer of V_convergence(S_k) for k in [t-W, t-1]
                             // stores the W PRECEDING values only — excludes current epoch
                             // initialized to 0 for all k < 0 (pre-genesis padding)
  halt_reason:   u8,         // 0x00=None (running); 0x01–0x06=halt codes; once non-zero, all transitions produce ⊥
)
```

**Validator cardinality bound:**

```
N_max = 1024   (protocol law, not configurable)
```

`N` at genesis must satisfy `N ≤ N_max`. The serialized validator array always
contains exactly `N` entries; unused slots do not exist. `N` is immutable after genesis.

Each validator record `V_i` is a tuple:

```
V_i = (
  id:         [u8; 48],   // STABLE public-key-derived validator identity
                           // = H_domain(VALIDATOR_ID, public_key_bytes)[0..48]
                           // Fixed at genesis; never changes across epochs.
                           // Anchors slash accounting continuity.
  divergence: i64,        // D_i,t ∈ [0, SCALE]; deviation metric, defined in §4a
  conflict:   i64,        // C_i,t ∈ [0, SCALE]; conflict metric, defined in §4a
  slash_acc:  i64,        // Σ_i,t ≥ 0 (monotone non-decreasing); defined in §4b
  nonce:      u64,        // per-author replay counter; advanced on each admitted TX
)
```

> **Implementation note**: In the Rust reference implementation, `id` and `nonce`
> are stored as parallel arrays (`EpochState.validator_ids` and `EpochState.nonces`)
> rather than inside `ValidatorMetrics`, for cache efficiency. The spec presents all
> fields together under `V_i` for conceptual clarity. The wire encoding interleaves
> fields per validator slot (divergence, conflict, slash_acc, nonce, id) as defined
> in §2. There is no semantic difference between the two representations.

> **Validator identity vs Merkle leaf index:**
> `V_i.id` is the validator's **stable consensus identity** — a 48-byte
> truncated hash of the validator's public key, fixed at genesis. It never
> changes across epochs and anchors slash accounting continuity.
>
> The `obfuscation` section of `GENESIS_CONSTANTS.toml` defines a separate
> **Merkle leaf index** construction: `validator_id(8) ∥ epoch(8) ∥ seed(32)`.
> This 48-byte epoch-relative concatenation is used exclusively for sparse
> Merkle tree leaf addressing — not for validator consensus identity. The two
> constructions are distinct and must not be conflated. Epoch-relative Merkle
> leaf indices change each epoch; the consensus identity `V_i.id` never does.

### Admissibility constraints

A state `S_t` is **admissible** if and only if:

```
1. S_t.epoch is strictly greater than S_{t-1}.epoch
2. S_t.halt_reason = 0x00 (None)  (halt states are terminal; any non-zero halt_reason is inadmissible)
3. ∀ i: S_t.validators[i].divergence ∈ [0, i64::MAX]  (divergence is non-negative)
4. ∀ i: S_t.validators[i].slash_acc ∈ [0, i64::MAX]   (slash accumulator is non-negative)
5. S_t.entropy_seed ≠ [0u8; 32]  (zero seed is forbidden post-genesis)
6. S_t.state_root = H_consensus_domain(STATE_ROOT=0x00000001, Encode_for_commitment(S_t, prior_root(t)))
   This is the canonical commitment invariant defined in §2.
   prior_root(t) = S_{t-1}.state_root for t≥1; [0u8;32] for t=0.
```

---

## §2 — Canonical Encoding

> **ENCODING FREEZE DECLARATION**
>
> Canonical encoding is consensus identity.
> `Encode(S_t)` as defined in this section is permanently frozen.
>
> Any modification to the wire format — field order, field width, endianness,
> padding, domain tags, or type representation — constitutes a new protocol
> universe and requires a new genesis. There are no "compatible upgrades" to
> encoding. Serialization is ontology for a replay-first protocol: two states
> that encode identically are the same state; two protocols that encode
> differently are different protocols.
>
> The SHA3-256 of this document is recorded in `GENESIS_CONSTANTS.toml`
> at genesis lock time. Any runtime that implements a different encoding
> is not a QASH implementation regardless of other conformance.

### Encoding vs state root computation

`Encode(S_t)` encodes the state **as-is**, including the current `state_root` field.
This is used for: wire transmission, storage, and replay.

### Prior root as a total function

```
prior_root : ℕ → [u8; 32]
prior_root(0)     = [0u8; 32]
prior_root(t + 1) = S_t.state_root
```

This is the authoritative definition. All references to "prior root" use `prior_root(t)`.
The function is total over all epoch indices, supporting structural induction in Coq.

### substitute_root

```
substitute_root(S : State, new_root : [u8; 32]) → State
  S' where S'.state_root = new_root
           S'.f = S.f  for all field f ≠ state_root
```

`substitute_root` is a pure record update. Then:

```
Encode_for_commitment(S, prior_root) := Encode(substitute_root(S, prior_root))
```

### Encoding ontology (normative)

> **No other encoding variants are permitted anywhere in this protocol.**
>
> | Use case | Function |
> |----------|----------|
> | Transmission, storage, replay, equality, decoding, TH-1 | `Encode` |
> | State root commitment (transition step 9 only) | `Encode_for_commitment` |
>
> Two states are consensus-identical iff `Encode(S_a) = Encode(S_b)`.
> Future documents must not introduce alternative serializations or zeroing variants.

`Encode_for_commitment(S, prior_root)` is a **commitment preimage constructor**.
It is NOT an alternative wire encoding — `Encode` remains the sole canonical wire format.
`Encode_for_commitment` is a deterministic state transformation used solely before hashing:

```
Encode_for_commitment(S, prior_root) := Encode(substitute_root(S, prior_root))
```

This is the **sole normative definition**. The informal notation
`Encode(S with state_root := prior_root)` is non-normative shorthand only
and must not be treated as a separate semantic object in proofs or implementations.

The state root is then:

```
state_root_t = H_consensus_domain(STATE_ROOT, Encode_for_commitment(S_t, prior_root(t)))
```

**Key invariant:** The wire encoding of S_t always contains the commitment to S_{t-1}.
The `state_root` field binds S_t to the prior state, creating a verifiable chain
analogous to blockchain header chaining.

**Genesis:** `S_0.state_root = [0u8; 32]` (prior_root at genesis = all-zeros).

**Why not zeroing:** Zeroing (the `Encode_canonical_inputs` approach) is equivalent
at genesis but loses chain history at t > 0. Prior_root substitution makes chaining
explicit and preserves the "Encode = identity" invariant.

`Encode(S_t)` produces a deterministic byte sequence.
This encoding is **protocol law**: any deviation is a consensus failure, not an implementation bug.

### Wire format

All fields are serialized in the order listed. No padding. No alignment gaps.
All integers are **little-endian**.

```
Encode(S_t):                                             — 112 fixed bytes, then validators, then window

  epoch:           u64     →  8 bytes, little-endian
  state_root:      [u8;32] → 32 bytes, verbatim
  ledger_root:     [u8;32] → 32 bytes, verbatim
  entropy_seed:    [u8;32] → 32 bytes, verbatim
  halt_reason:     u8      →  1 byte  (valid values: 0x00–0x06; any other value is malformed)
                               0x00  None              (running)
                               0x01  LyapunovViolation (H1)
                               0x02  ArithOverflow     (H2)
                               0x03  EpochOverflow     (H3)
                               0x04  DecodeInvalid     (H4)
                               0x05  RoundtripFailure  (H5)
                               0x06  HaltFlagSet       (H6, reserved)
  pad:             [u8; 3] →  3 bytes, must be 0x00 0x00 0x00; non-zero is malformed
  validator_count: u32     →  4 bytes, little-endian, must equal N from genesis
                           — subtotal: 8+32+32+32+1+3+4 = 112 bytes
  validators:      [Encode(V_i); N] → N × 80 bytes, concatenated, no separators

Encode(V_i):                                             — 80 bytes per validator

  divergence:      i64     →  8 bytes, little-endian, two's complement
  conflict:        i64     →  8 bytes, little-endian, two's complement
  slash_acc:       i64     →  8 bytes, little-endian, two's complement
  nonce:           u64     →  8 bytes, little-endian
  id:              [u8;48] → 48 bytes, verbatim
                           — subtotal: 8+8+8+8+48 = 80 bytes

Lyapunov window:                                         — 28 bytes

  window_filled:   u8      →  1 byte (∈ [0, W]; value > W is malformed)
  pad:             [u8; 3] →  3 bytes, must be 0x00 0x00 0x00; non-zero is malformed
  lyapunov_window: [i64; W] → W × 8 bytes, little-endian, two's complement, W=3
                           — subtotal: 1+3+24 = 28 bytes
```

**Total encoded size** (fixed and computable at genesis from N and W):

```
|Encode(S_t)| = 112 + N × 80 + 28  bytes
```

Variable-length encoding is forbidden.

### Deterministic rejection

Any input byte sequence that:
- has incorrect total length,
- contains a `bool` field with value other than `0x00` or `0x01`,
- has `validator_count ≠ N`,
- or violates any admissibility constraint after decoding

**must** produce an identical, deterministic rejection result on all platforms:

```
DecodeResult::Invalid(reason: u8)
```

where `reason` is a protocol-defined error code. The error code is part of the canonical state.

### Genesis hash procedure (normative)

At genesis lock time, the SHA3-256 of the canonical spec document set is computed and
recorded in `GENESIS_CONSTANTS.toml` as `genesis_hash`. This commits the genesis network
to an immutable document tree. `GENESIS_CONSTANTS.toml` itself is excluded to avoid circularity.

**Document set** (concatenated in lexicographic file-path order):

```
docs/spec/00_execution_model.md
docs/spec/01_consensus.md
docs/spec/02_transition_axioms.md
docs/spec/03_transactions.md
```

**Computation:**

```sh
python3 -c "
import hashlib, pathlib
files = sorted([
    'docs/spec/00_execution_model.md',
    'docs/spec/01_consensus.md',
    'docs/spec/02_transition_axioms.md',
    'docs/spec/03_transactions.md',
])
h = hashlib.sha3_256()
for f in files:
    h.update(pathlib.Path(f).read_bytes())
print('SHA3-256:' + h.hexdigest())
"
```

**Format in `GENESIS_CONSTANTS.toml`:** `genesis_hash = "SHA3-256:<64 lowercase hex digits>"`

Any subsequent modification to the above four documents constitutes a new genesis and
requires recomputing this value.

---

## §3 — Transition Function

```
T : (S_t, I_t) → S_{t+1} | ⊥
```

### Input set

`I_t` is the canonical input at epoch `t`:

```
I_t = (
  transactions:  [Tx; M],    // M ≤ max_queries_per_epoch from genesis
  validator_sigs: [Sig; N],  // one Dilithium5 signature per active validator
  epoch_anchor:  [u8; 32],   // SLH-DSA-SHA3-256 anchor over I_t \ {epoch_anchor}
)
```

`I_t` is admissible if and only if:
- all signatures verify under their respective validator public keys,
- `epoch_anchor` verifies under the epoch anchor key,
- `M ≤ max_queries_per_epoch`,
- **`I_t.transactions` is canonically ordered before admission into Domain A.**

The canonical ordering is defined in `02_transition_axioms.md §A10`:
transactions are sorted by `H_domain(ENTROPY_ADVANCE, S_t.entropy_seed ∥ Encode(τ))`.
An `I_t` whose transactions are not in this order is inadmissible and triggers
absorbing halt. This closes the determinism leak where two validators could
admit semantically equivalent but differently-ordered transaction batches.

### Transition steps

Given admissible `(S_t, I_t)`:

```
1. If S_t.halt_reason ≠ None (0x00):  return ⊥

2. Apply transactions:
   S' ← ApplyAll(S_t, I_t.transactions)
   If any transaction triggers overflow_policy:  return ⊥

3. Advance entropy:
   S'.entropy_seed ← SHA3-256(S_t.entropy_seed)

4. Compute convergence potential:
   v ← V_convergence(S')        // defined in §4a

5. Compute safety accumulator:
   φ ← Φ_safety(S')             // defined in §4b

6. Check stability criterion (§5) BEFORE updating the window:
   // Window contains preceding W values; current v is not yet in it
   If δ_window(v, S_t.lyapunov_window) > ε OR φ ≥ Φ_max_safe:
                          S'.halt_reason ← LyapunovViolation (0x01)
                          return S' with halt_reason set (absorbing)

7. Update Lyapunov window with current value:
   S'.lyapunov_window ← rotate_left(S_t.lyapunov_window, v)
   // Now window stores [S_{t-W+1}, ..., S_t] for next epoch's check

8. Increment epoch:
   S'.epoch ← S_t.epoch + 1

9. Compute new state root using prior-root substitution (LAST step):
   S'.state_root ← H_consensus_domain(STATE_ROOT=0x00000001, Encode_for_commitment(S', prior_root(t+1)))
   // prior_root(t+1) = S_t.state_root by definition (§2)

10. return S'
```

All steps are pure functions over fixed-size, statically-bounded data structures.
No unbounded or nondeterministically-sized allocation is permitted.
See 00_execution_model.md §E3 for the full allocation policy.

---

## §4 — Stability Functions

The protocol uses **two structurally distinct functions** with separate proof obligations.
They must not be combined. Combining them was an identified design flaw: a monotone
non-decreasing term inside a convergence function invalidates all convergence proofs.

---

### §4a — Operational Lyapunov Candidate `V_convergence`

> **Scope:** Dynamical convergence analysis only.
> **Proof obligation:** `ΔV_convergence ≤ ε` under all admissible honest transitions.
> **Proof target:** `proofs/contractivity/lyapunov_stability.v`

`V_convergence` is a candidate Lyapunov function over the **dynamic** validator state.
It contains only terms that can both increase and decrease, enabling convergence analysis.

```
V_convergence(S_t) = Σ_i [ α · D_i,t  +  β · C_i,t ]
```

where the sum is over all active validators.

| Symbol | Genesis constant | Value | Semantic meaning |
|--------|-----------------|-------|-----------------|
| `α` | `weight_divergence_D` | `400_000` | Weight on per-validator state divergence |
| `β` | `weight_conflict_C` | `350_000` | Weight on per-validator conflict density |

**Component definitions:**

`D_i,t` — **Divergence**: normalized Hamming distance between validator `i`'s
committed state root and the epoch consensus root.

Hamming distance over 32-byte (256-bit) hash outputs is semantically meaningful:
it counts bitwise disagreement. It is deterministic, architecture-independent,
bounded in `[0, 256]`, and preserves cryptographic opacity while measuring
disagreement density. Arithmetic distance on hash prefixes has none of these
properties and must not be used.

```
XOR_root_i,t  = V_i.committed_root XOR S_t.state_root   (bitwise, 32 bytes)
hamming_i,t   = popcount(XOR_root_i,t)                   (count of differing bits, ∈ [0, 256])
D_i,t         = hamming_i,t × p / 256                    (normalized to [0, p])
```

`popcount` is the number of set bits across all 32 bytes of the XOR result.
It is computed as the sum of per-byte bit counts using a constant-time lookup table
(256-entry, precomputed at genesis, part of the deterministic domain).

If validator `i` has not submitted a root for the current epoch: `D_i,t = p` (maximum divergence).

**Determinism note:** `popcount` on fixed-width byte arrays is bitwise-identical across
all authorized ISAs. Hardware `POPCNT` instructions must only be used if their output
has been verified to match the lookup-table reference implementation in the cross-ISA
test suite. Until verified, the lookup table is the authoritative implementation.

`C_i,t` — **Conflict density**: normalized count of conflicting transitions submitted
by validator `i` within the current evaluation window.

```
C_i,t = conflict_count_i × p / max_queries_per_epoch
result ∈ [0, p]
```

**Arithmetic contract for `V_convergence`:**

```
V_convergence(S_t):
  acc: i128 ← 0
  for each active V_i:
    term: i128 ← (α as i128) × (D_i,t as i128)
                + (β as i128) × (C_i,t as i128)
    if term > i128::MAX − acc: trigger absorbing_halt()
    acc ← acc + term
  return floor_div(acc, p as i128)   // floor_div defined in 00_execution_model.md §E1
```

`V_convergence(S_t) ∈ [0, N_max × (α + β) × p / p]` = `[0, N_max × (α + β)]`
for all admissible states. This bound is structurally enforced by type widths.

---

### §4b — Safety Accumulator `Φ_safety`

> **Scope:** Monotone safety witness. Not a convergence function.
> **Proof obligation:** Boundedness and halt-threshold correctness only.
> **Proof target:** `proofs/safety/absorbing_halt.v`

`Φ_safety` accumulates irreversible slash evidence. It is **monotone non-decreasing
by construction**. It does not converge to an equilibrium — it approaches a structural
upper bound that, when crossed, makes continuation inadmissible.

```
Φ_safety(S_t) = Σ_i [ γ · Σ_i,t ]
```

| Symbol | Genesis constant | Value | Semantic meaning |
|--------|-----------------|-------|-----------------|
| `γ` | `weight_slash_Sigma` | `250_000` | Weight on per-validator slash accumulation |

`Σ_i,t` — **Slash accumulator**: monotone non-decreasing running sum of slash events
for validator `i`. Never decreases. Saturates at `INT_MAX`.

```
INT_MAX  := 2^63 − 1          (pure arithmetic; maps to i64::MAX in Rust)

Σ_i,0    = 0                  (genesis)
Σ_i,t+1  = if Σ_i,t + slash_increment_i,t > INT_MAX
            then INT_MAX       (explicit cap via pre-checked conditional, NOT saturation)
            else Σ_i,t + slash_increment_i,t
```

Note: this is not saturating arithmetic. It is a domain-bounded accumulator with
an explicit pre-checked ceiling. The distinction matters for proofs: the cap is
an admissibility decision, not an arithmetic behavior. The underlying addition
is checked — if `slash_increment_i,t` is itself out of bounds, that triggers
absorbing halt before the cap logic is reached.

**Derived halt bound** (not a genesis parameter — follows from the type space):

```
Φ_max    := N_max × γ × INT_MAX
           = 1024 × 250_000 × (2^63 − 1)
           ≈ 2.36 × 10^21

Φ_max_safe := Φ_max / 2       (halt triggers before representational exhaustion)
```

`Φ_max_safe < Φ_max` by construction. This ensures the halt condition is an
**admissibility violation**, not an overflow event. Overflow remains unreachable
by invariant, not by arithmetic accident.

**Arithmetic contract for `Φ_safety`:**

```
Φ_safety(S_t):
  acc: i128 ← 0
  for each active V_i:
    term: i128 ← (γ as i128) × (Σ_i,t as i128)
    if term > i128::MAX − acc: trigger absorbing_halt()
    acc ← acc + term
  return acc    // NOT floor-divided; compared directly to Φ_max_safe
```

**Admissibility constraint on slash increments:**

```
∀ i, t: slash_increment_i,t ∈ [0, INT_MAX]
```

If any `slash_increment_i,t` exceeds `INT_MAX`, the transition producing it is
**inadmissible** and triggers absorbing halt before the `Σ_i,t` update is attempted.
This ensures the `i128` intermediate sum in the cap rule never overflows:
`current (≤ INT_MAX) + increment (≤ INT_MAX) ≤ 2 × INT_MAX < i128::MAX`.

The source and derivation of `slash_increment_i,t` is defined in the transaction
semantics (`docs/spec/03_transactions.md`, pending). Until that document exists,
`slash_increment` is treated as an opaque non-negative value bounded by `INT_MAX`.

**Monotonicity invariant** (proof obligation for `absorbing_halt.v`):

```
∀ t, admissible S_t, admissible I_t:
  Φ_safety(T(S_t, I_t)) ≥ Φ_safety(S_t)
```

This follows directly from the explicit pre-checked cap definition of `Σ_i,t`:
`Σ_i,t+1 ≥ Σ_i,t` holds because the cap is `INT_MAX` and `slash_increment ≥ 0`.

---

## §5 — Stability Criterion

The protocol advances an epoch if and only if **both** conditions below are satisfied.
Failure of either is an absorbing halt. The conditions are evaluated in order;
if condition 1 fails, condition 2 is not evaluated.

### Condition 1 — Convergence gate

Let `W = evaluation_window = 3`.

Let `preceding_window = [V_convergence(S_{t-W}), ..., V_convergence(S_{t-1})]`
— the `W` values **preceding** the current epoch, **excluding** `S_t` itself.

The **window excursion** (`δ_window`) measures how far the current potential has risen
above the minimum observed in the preceding evaluation window. This is **not** a temporal
derivative — it does not measure V_{t+1} − V_t. It measures rolling-minimum deviation:

```
δ_window = V_convergence(S_t) − min(preceding_window)
```

> Note: the window deliberately excludes the current epoch's value. Including it
> would create a self-minimizing edge case where a spike immediately becomes the
> new minimum, masking instability within the same window. Using only preceding
> values means a transient spike that recovers within W epochs will not trigger
> halt, but a sustained rise will.

During genesis initialization (`t < W`): `preceding_window` is padded with `0`
for all epochs before genesis. This ensures `δ_window ≥ 0` from epoch 0.

```
CONDITION 1 PASS  iff  δ_window ≤ ε      where ε = epsilon_threshold = 20_000
CONDITION 1 FAIL  iff  δ_window > ε      → absorbing halt
```

**Proof obligation:** `proofs/contractivity/lyapunov_stability.v` must show
`δ_window ≤ 0` under all admissible honest transitions (stronger than `≤ ε`).
`ε` provides tolerance for bounded perturbations only.

### Condition 2 — Safety admissibility gate

```
CONDITION 2 PASS  iff  Φ_safety(S_t) < Φ_max_safe
CONDITION 2 FAIL  iff  Φ_safety(S_t) ≥ Φ_max_safe  → absorbing halt
```

where `Φ_max_safe = Φ_max / 2 = N_max × γ × (2^63 − 1) / 2`.

This condition triggers before representational exhaustion by construction.
Overflow is unreachable as a protocol invariant, not merely by runtime check.

**Proof obligation:** `proofs/safety/absorbing_halt.v` must show:
- `Φ_safety` is monotone non-decreasing under all transitions
- `Φ_safety(S_0) = 0` at genesis
- `Φ_max_safe` is unreachable in finite epochs under bounded slash increments

### Combined criterion

```
STABLE   iff  δ_window ≤ ε  AND  Φ_safety(S_t) < Φ_max_safe
HALT     iff  δ_window > ε  OR   Φ_safety(S_t) ≥ Φ_max_safe
```

### Epoch gating invariant

No state `S_{t+1}` with `S_t.halt_reason ≠ None` is ever produced by an honest validator.
Any such transition must be rejected deterministically by all other validators.

---

## §6 — Replay Invariance Theorem Statement

This section states the theorem that all CI, test vectors, and cross-ISA verification
are designed to prove.

### Theorem RT-1 (Replay Invariance)

Let `G` be the genesis constants block.
Let `T = (I_0, I_1, ..., I_n)` be a canonical input sequence, where each `I_t` is admissible.
Let `p_x` and `p_y` be any two platforms from the Tier A authorized ISA set:

```
{ x86_64-avx2, aarch64-neon, riscv64-vector }
```

as defined in `docs/spec/00_execution_model.md §E6`. The theorem does not claim
bitwise identity across arbitrary hardware; it is bounded to this set only.

Then:

```
Replay_{p_x}(G, T) = Replay_{p_y}(G, T) = R_n
```

where `R_n = S_n.state_root` and the canonical commitment invariant (§2) holds:
`R_n = H_consensus_domain(STATE_ROOT, Encode_for_commitment(S_n, prior_root(n)))`

That is: deterministic re-execution of the canonical input sequence from genesis
produces a bitwise-identical state root on all authorized platforms.

### Proof obligations

RT-1 is not proven here. It is the **proof target** for:

- The cross-ISA CI workflow (`platform-determinism.yml`)
- The deterministic test vector suite (to be defined in `docs/spec/02_test_vectors.md`)
- The Coq contractivity proof (`proofs/contractivity/lyapunov_stability.v`)
- The TLA+ safety invariant (`proofs/safety/`)

RT-1 depends on:
1. §0 constraints being enforced (no float, no nondeterminism)
2. §2 canonical encoding being unique (no two distinct `S_t` produce the same encoding)
3. §3 transition function being total and pure
4. §4 Lyapunov computation using only `i128` intermediate arithmetic with floor rounding
5. §5 stability criterion applying identically across platforms

### Corollary RT-2 (Succession Soundness)

If the network halts at epoch `n` (halt_reason ≠ None), and a successor network `G'`
anchors to `R_n` as its genesis state root, then:

```
S'_0.state_root = R_n
```

is the unique valid starting condition for `G'`. No other genesis root is accepted
by an honest validator of `G'`.

---

## §7 — Theorem Dependency Graph

All protocol guarantees reduce to the following proof graph.
Each node is a theorem or axiom. Edges are dependency arrows (A → B means B depends on A).
No implementation claim is valid until its proof obligations are discharged.

### Axiom layer (class: ASSUMED — trusted, not proved within the system)

```
AX-1  ISA correctness:   [ASSUMED] authorized ISAs implement two's complement correctly
AX-2  Compiler:          [ASSUMED] pinned Rust toolchain produces correct code
AX-3  Hash security:     [ASSUMED] the active consensus hash suite (SHA3-256 + SM3-256, folded by SHA3-256) is modeled as collision-resistant over protocol state space.
                                   IMPORTANT: no fixed-width hash root is mathematically injective
                                   (collisions exist by pigeonhole). This axiom assumes
                                   collisions are computationally unreachable within the
                                   protocol's admissible state space. It is a computational
                                   assumption modeled as a mathematical axiom. Named
                                   AX3_sha3_assumed_injective in the Coq proof files to
                                   make the trust class explicit.
```

### Theorem and verification claim layer

Two classes exist. They are epistemically distinct and must not be conflated:

- **FORMAL THEOREM** — machine-proved from axioms; deductive certainty given AX-1/AX-2/AX-3
- **VERIFICATION CLAIM** — validated by CI/test vectors; empirical evidence, not proof

```
TH-1  Encoding injectivity
      Encode(x) = Encode(y) ⇒ x = y
      Depends on: AX-1, AX-2
      Class: FORMAL THEOREM
      Proof file: proofs/contractivity/encode_injectivity.v
      Status: FORMAL — Coq compiles; zero Admitted beyond AX-1/AX-2

TH-2  Encoding totality
      Encode is defined for all admissible S_t
      Depends on: AX-1, AX-2
      Class: FORMAL THEOREM
      Proof file: proofs/contractivity/encode_injectivity.v (co-located)
      Status: PROVED

TH-3  Convergence non-increase (revised — see note)
      ∀ admissible honest (S_t, I_t): δ_window(T(S_t, I_t)) ≤ ε_honest
      where ε_honest = 2_000  (see two-threshold model below)
      Depends on: TH-1, AX-1, AX-2, §A8 proof obligations for all admitted τ
      Class: FORMAL THEOREM
      Proof file: proofs/contractivity/lyapunov_stability.v
      Status: FORMAL — proofs/contractivity/lyapunov_stability.v; zero Admitted beyond AX-1/AX-2

      NOTE on target strength: the original target δ_window ≤ 0 (strict
      non-increase per epoch) is too strong for any nontrivial transaction
      system — the protocol explicitly permits bounded perturbation via
      §A8 Form B. The revised target δ_window ≤ ε_honest provides:
        - a mathematically achievable convergence guarantee
        - a safety margin: ε_honest (2_000) << ε_halt (20_000)
        - compatibility with §A8 Form B perturbation budgets

      Two-threshold model:
        ε_honest = 2_000  — proof target; honest transactions must stay within
        ε_halt   = 20_000 — halt trigger; ε_honest << ε_halt gives safety margin
        ratio    = 10×    — ten epochs of full perturbation before halt

      TH-3 proof strategy (via §A8 composition):
        For each admitted τᵢ: δ_window(τᵢ) ≤ δ_window + ε_τᵢ  (§A8 obligation)
        Σ ε_τᵢ ≤ ε_honest per epoch                              (epoch budget)
        ∴ δ_window(ApplyAll(S_t, I_t)) ≤ δ_window(S_t) + ε_honest

TH-3a Halt determinism (corollary of RT-1)
      If Replay(G, T) derives halt_reason ≠ None at epoch n on one honest
      validator, then every honest validator replaying the same admissible T
      must derive the same halt_reason at epoch n.
      Depends on: TH-7 (RT-1 replay invariance), TH-6
      Class: FORMAL THEOREM (follows from RT-1 applied to halt_reason field)
      Proof: halt_reason is part of Encode(S_n); RT-1 guarantees identical
             Encode(S_n) on all honest Ξ; therefore identical halt_reason.
      Status: STATED (proof follows from RT-1 composition)

TH-4  Φ_safety monotonicity
      ∀ admissible (S_t, I_t): Φ_safety(T(S_t, I_t)) ≥ Φ_safety(S_t)
      Depends on: AX-1, AX-2
      Class: FORMAL THEOREM
      Proof file: proofs/safety/absorbing_halt.v
      Status: FORMAL — proofs/safety/absorbing_halt.v; zero Admitted beyond AX-1/AX-2

TH-5  Φ_safety boundedness
      ∀ admissible S_t: Φ_safety(S_t) ≤ Φ_max
      Depends on: TH-4, AX-1
      Class: FORMAL THEOREM
      Proof file: proofs/safety/absorbing_halt.v
      Status: FORMAL — proofs/safety/absorbing_halt.v; zero Admitted beyond AX-1/AX-2

TH-6  Halt correctness
      halt_reason ≠ None ⇒ no further admissible transitions exist
      Depends on: TH-4, TH-5
      Class: FORMAL THEOREM
      Proof file: proofs/safety/absorbing_halt.v
      Status: FORMAL — proofs/safety/absorbing_halt.v; zero Admitted beyond AX-1/AX-2

TH-7  Replay invariance (RT-1)
      ∀ ISA ∈ {x86_64, aarch64, riscv64}:
        Replay_ISA(G, T) = R_n
      Depends on: TH-1, TH-2, AX-1, AX-2
      Class: VERIFICATION CLAIM (empirical evidence, not deductive proof)
             platform-determinism.yml provides evidence; full proof requires TH-1 discharge
      Verification: platform-determinism.yml + test vectors (docs/spec/07_test_vectors.md)
      Status: PARTIAL — CI-verified on x86_64; aarch64 and riscv64gc cross-ISA runs pending

TH-8  Succession soundness (RT-2)
      S'_0.state_root = R_n is the unique valid genesis for successor G'
      Depends on: TH-1, TH-6, AX-3
      Class: FORMAL THEOREM
      Proof file: proofs/safety/absorbing_halt.v
      Status: FORMAL — proofs/integration/th8_composition.v; zero Admitted beyond AX-1/AX-2/AX-3
```

### Dependency graph (ASCII)

```
AX-1 ──┬──────────────────────────────────────┐
        │                                      │
AX-2 ──┼──────────────────────┐               │
        │                      │               │
        ▼                      ▼               ▼
      TH-1 ──► TH-2      TH-4 ──► TH-5 ──► TH-6
        │         │         │                  │
        │         └────┬────┘                  │
        ▼              ▼                        ▼
      TH-3           TH-7                    TH-8
                                               ▲
AX-3 ─────────────────────────────────────────┘
```

### Genesis lock gate

`GENESIS_CONSTANTS.toml` must not be locked until:
- TH-1, TH-2, TH-3, TH-4, TH-5, TH-6, TH-8: FORMAL (Coq compiles; zero Admitted beyond AX-1/AX-2/AX-3)
- TH-7: CI-verified on x86_64; aarch64 and riscv64gc cross-ISA runs must pass before final lock
- `genesis_hash` in `GENESIS_CONSTANTS.toml` must be set to the SHA3-256 of the canonical spec document set (see §2 genesis hash procedure)

---

## Appendix A — Binding of Genesis Constants

The following table maps every consensus-relevant field in `GENESIS_CONSTANTS.toml`
to its formal definition in this specification. Fields marked **derived** are not
genesis parameters — their values follow necessarily from the listed sources.

| TOML field / constant | § | Formal role | Proof dependency |
|----------------------|---|-------------|-----------------|
| `fixed_point.scale` | §0, §4a | `p = 1_000_000`; denominator of `𝔽_p` | TH-1 |
| `fixed_point.intermediate_width` | E1 | Mandates `i128` for all intermediates | TH-1, TH-3 |
| `fixed_point.rounding_mode` | E1 | `floor_div` toward −∞ | TH-3 |
| `fixed_point.overflow_policy` | §0, §3 | Absorbing halt on `i128` overflow | TH-6 |
| `lyapunov.weight_divergence_D` | §4a | `α = 400_000` | TH-3 |
| `lyapunov.weight_conflict_C` | §4a | `β = 350_000` | TH-3 |
| `lyapunov.weight_slash_Sigma` | §4b | `γ = 250_000` | TH-4, TH-5 |
| `lyapunov.epsilon_threshold` | §5 | `ε = 20_000` | TH-3 |
| `lyapunov.evaluation_window` | §5 | `W = 3` | TH-3 |
| `lyapunov.max_queries_per_epoch` | §3, §4a | Bounds `M`; normalizes `C_i,t` | TH-3 |
| `epoch.timing.duration_ms` | E6 | Wall-clock budget (nondeterministic domain, PAL only) | none |
| `obfuscation.leaf_index_bytes` | §1 | `V_i.id` width = 48 bytes | TH-1 |
| `crypto.cascade.primary_signature` | §3 | Dilithium5 per-validator signatures | AX-2 |
| `crypto.cascade.anchor_signature` | §3 | SLH-DSA-SHA3-256 epoch anchor | AX-2 |
| `N_max = 1024` | §1 | **Protocol law** (not TOML); max validator cardinality | TH-1, TH-5 |
| `INT_MAX = 2^63−1` | §4b | **Derived** from i64 width; not a TOML field | TH-5 |
| `Φ_max` | §4b | **Derived**: `N_max × γ × INT_MAX` | TH-5 |
| `Φ_max_safe` | §5 | **Derived**: `Φ_max / 2` | TH-6 |

Any genesis field not listed here has no direct bearing on `T`, `V_convergence`,
`Φ_safety`, or `Encode` and is an operational parameter only.

## Appendix B — Relationship to `00_execution_model.md`

This document defines **what** the protocol computes.
`00_execution_model.md` defines **how** it is permitted to compute it.

Both documents are protocol law. In case of conflict, `00_execution_model.md` governs
execution constraints; this document governs semantic correctness.

---

*End of `docs/spec/01_consensus.md`*
*SHA3-256 of this document to be recorded in `GENESIS_CONSTANTS.toml` at lock time.*
