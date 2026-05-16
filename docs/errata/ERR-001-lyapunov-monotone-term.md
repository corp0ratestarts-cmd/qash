# ERR-001: Lyapunov Monotone Term in Convergence Candidate

- **Status:** accepted
- **Normative source:** `spec/pdf/QASH_Spec_v1.0.pdf`, §4.1, pp. 9–10
- **Traceability rows:** P0-1

## Verbatim PDF text

```text
pub struct LyapunovState {
    divergence: FixedPoint,
    conflict: FixedPoint,
    signature_health: FixedPoint,
}
```

```text
L = W_D·D + W_C·C + W_S·Σ
```

```text
∀t: L(t+1) - L(t) ≤ ε_threshold
```

```text
If violated → absorbing halt
```

## Ambiguity

The PDF does not specify whether `signature_health` (`Σ`) is monotone
(cumulative penalty evidence) or non-monotone (recoverable verification status).
The field comment says only `Cryptographic primitive verification status`, while
the genesis parameter name `weight_slash_Sigma` points toward slash/penalty
semantics.

If `Σ` is monotone, placing `W_S·Σ` inside the convergence candidate can make
`L` increase under otherwise valid operation. That would conflate convergence
control with safety evidence and can produce false halts even as divergence and
conflict improve.

## Resolution: two-function partition

Partition the PDF's total energy expression into:

- `V_convergence(t) = W_D·D(t) + W_C·C(t)` — dynamic terms checked by the
  δ-window convergence gate.
- `Φ_safety(t) = W_S·Σ_aggregate(t)` — monotone safety evidence checked by a
  separate safety gate once ADR-001 and ADR-002 define its threshold and
  aggregation rule.

The PDF's `L = W_D·D + W_C·C + W_S·Σ` remains interpretable as total energy:
`L(t) = V_convergence(t) + Φ_safety(t)`. The convergence proof obligation applies
to `V_convergence`, not to the monotone safety accumulator.

## New parameter required

`Φ_max_safe` or an equivalent safety threshold must be defined and pinned before
genesis if the separate safety gate is adopted. ADR-001 records this PDF-silent
extension.

## Impact

- P0-1 is unblocked from the erratum side, but remains blocked on ADR-001 and
  ADR-002 until `Φ_safety` aggregation and threshold semantics are accepted.
- P0-9 remains blocked until the new safety threshold is either defined or the
  safety gate is explicitly rejected.
- `proofs/contractivity/lyapunov_stability.v` must prove convergence properties
  for `V_convergence` only.
