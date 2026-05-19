# Halt Taxonomy

This document defines the formal taxonomy for `HaltReason` transitions in Domain A consensus.

| HaltReason | Trigger | Scope | Expected transition effect | Replay consequences |
|---|---|---|---|---|
| `None (0x00)` | No invariant violation and no halt latch set. | Local epoch transition. | Transition proceeds; epoch may increment and state root updates deterministically. | Replays remain active and must be bit-identical under deterministic inputs. |
| `LyapunovViolation (0x01)` | Candidate Lyapunov score rises above permitted delta window. | Consensus safety / convergence. | Transition returns `Err`, sets halt latch to `LyapunovViolation`, and blocks further progress. | Replay becomes absorbing; all future transitions return same halt reason. |
| `ArithOverflow (0x02)` | Checked arithmetic overflow in fixed-point or transition arithmetic paths. | Arithmetic soundness in transition core. | Transition aborts, latches halt. | Replay is terminal and deterministic (`ArithOverflow` repeats). |
| `EpochOverflow (0x03)` | Epoch increment / chain validation overflows `u64`. | Epoch sequencing and monotonic clocking. | Transition aborts; halt latch records epoch overflow. | Replay terminal; cannot advance epoch beyond overflow boundary. |
| `DecodeInvalid (0x04)` | Invalid envelope/state decode, non-canonical bytes, bound or pad violations. | Wire/state decoding boundary. | Transition aborts; halt latch set to decode fault. | Replay terminal for that state image; deterministic decoder rejects same payloads. |
| `RoundtripFailure (0x05)` | Internal encode/decode roundtrip self-check fails. | State encoding integrity. | Transition halts immediately to preserve safety. | Replay terminal and consistent for same bytes/logic. |
| `HaltFlagSet (0x06)` | External halt already latched on input state. | Global state latch. | No-op transition result with existing halt reason preserved. | Replay remains absorbing; all further calls return `HaltFlagSet`. |
| `PhiSafetyViolation (0x07)` | Phi-safety check fails. | Safety invariant for validator divergence/conflict envelope. | Transition aborts and latches halt. | Replay terminal with stable reason. |
| `IncompatibleVersion (0x08)` | Legacy (v1.0) envelope used after compatibility cutoff. | Version compatibility boundary. | Transition rejects envelope and latches incompatible version halt. | Replay terminal for same post-cutoff legacy payloads. |

## Governance rule

Any change to `HaltReason` enum membership, discriminant, or semantics **MUST** update:

1. This taxonomy document.
2. Halt roundtrip tests.
3. Vector ledger entries describing the new/changed code.
