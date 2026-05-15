# Threat Model: Nondeterminism and Replay Poisoning

## Scope

This threat model covers values or behaviors that can cause two conforming
operators to compute different state roots from the same logical epoch input.
It focuses on Domain A consensus execution and the Domain B boundary.

## Assets

- Replay-invariant state transition `T(S_t, I_t)`.
- Canonical state encoding and state roots.
- Validator metrics used by convergence evaluation.
- Absorbing halt reasons.
- Cross-ISA equivalence evidence.

## Threat vectors

| Threat | Example | Required mitigation |
| --- | --- | --- |
| Clock contamination | Wall-clock time changes epoch transition semantics | Domain B clock data cannot enter Domain A transition inputs |
| Entropy contamination | OS randomness influences validator update order | Entropy is forbidden in Domain A and must be rejected before admissibility |
| Network-order contamination | Arrival order changes state order | Candidate inputs must be canonically encoded and admissibility-checked |
| Architecture skew | Endianness or integer width changes encoded bytes | Fixed-width integer types and canonical encoding only |
| Arithmetic divergence | Overflow, wrapping, or floating-point rounding differs by target | Checked integer/fixed-point arithmetic; no floats in Domain A |
| Iteration nondeterminism | Hash-map ordering changes validator updates | Deterministic containers and static slot ordering |
| Hardware acceleration skew | Optimized crypto returns different consensus-visible bytes | Acceleration remains in Domain B unless output equivalence is verified |
| Replay poisoning | Malformed historical input causes different halt behavior | Decode invalidity maps to deterministic rejection or absorbing halt |

## Security invariants

1. Domain B values do not influence Domain A transition semantics.
2. Domain A arithmetic is checked and deterministic.
3. Encoding is canonical and independent of host architecture.
4. Halt conditions are deterministic and replay-visible.
5. Cross-target replay artifacts must produce identical roots for the same
   admissible input sequence.

## Evidence tasks

- Add negative tests for clock, entropy, and trailing-slot contamination.
- Archive replay equivalence artifacts for authorized targets.
- Add mutation tests for endian and arithmetic rule violations.
- Add CI checks that reject forbidden Domain A APIs.
