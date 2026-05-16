# ERR-001 — Lyapunov function mixes convergence + safety in a single halt predicate
**Status:** Accepted (Option B: two-function partition)  
**Filed:** 2026-05-13  
**Affects:** P0-1

## PDF citation
§4.1 (pp. 9–10):
> ```rust
> pub struct LyapunovState {
>     divergence: FixedPoint,
>     conflict: FixedPoint,
>     signature_health: FixedPoint, // Cryptographic primitive verification status
> }
> ```
> `L = W_D·D + W_C·C + W_S·Σ`  
> `∀t: L(t+1) - L(t) ≤ ε_threshold — If violated → absorbing halt`

§4.1 genesis TOML (p. 10):
> ```toml
> [lyapunov]
> weight_divergence_D = 400_000   # 0.40
> weight_conflict_C   = 350_000   # 0.35
> weight_slash_Sigma  = 250_000   # 0.25
> epsilon_threshold   =  20_000   # 0.02
> convergence_window  = 3
> ```

## Ambiguity
The PDF does not define whether `signature_health` (`Σ`) is monotone (cumulative penalty)
or non-monotone (recoverable "status"), nor does it define update rules.

## Risk
If `Σ` is monotone non-decreasing (a plausible reading given `weight_slash_Sigma`),
then using `L(t+1)-L(t)` as a single convergence/halt predicate conflates:
- **Convergence** (terms expected to decrease under healthy operation), and
- **Safety accounting** (terms that may only increase as evidence accumulates).

This can make the halt predicate sensitive to cumulative accounting rather than convergence.

## Resolution (Option B)
Partition the PDF's `L(t)` into two functions evaluated with separate gates:

- `V_convergence(t) = W_D·D(t) + W_C·C(t)`  (convergence candidate)
- `Φ_safety(t)      = W_S·Σ_aggregate(t)`   (safety accumulator)

And define:
- **H1 (refines PDF §4.1 halt):** apply the PDF convergence check to `V_convergence` only.
- **H2 (extension):** gate `Φ_safety` by `phi_max_safe` (defined by ADR-001).

The identity `L(t) = V_convergence(t) + Φ_safety(t)` is preserved.

## Halt conditions
| ID | Condition | Source |
|---|---|---|
| H1 | PDF §4.1 halt check, applied to `V_convergence` | Refines PDF |
| H2 | `Φ_safety(t) ≥ phi_max_safe` | ADR-001 (new) |

## Impact on traceability
- P0-1: unblocks assessing **what** the convergence gate applies to (V only).
- P0-9: genesis hash remains blocked until ADR-001 pins `phi_max_safe`.
