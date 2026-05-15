# Invention Disclosure: Lyapunov-Based Consensus Stability Evaluation

## Problem statement

Validator-set health can degrade gradually through divergence, conflicting
observations, or accumulated slash evidence. Conventional systems often respond
through ad hoc thresholds, governance processes, or post-hoc slashing, which may
not provide deterministic replay-stable stability evaluation.

## Prior-art limitations

Many consensus protocols track validator faults, but they do not necessarily
encode a deterministic Lyapunov-style convergence measure as part of the state
transition law, nor do they combine bounded metrics, fixed-point arithmetic,
windowed comparison, and absorbing halt semantics into a replay-verifiable
mechanism.

## Novel mechanism

QASH computes validator stability with deterministic fixed-point metrics:

- divergence `D` in a bounded interval,
- conflict `C` in a bounded interval,
- monotone slash accumulator `Σ`,
- weighted convergence score `V_convergence`,
- safety monitor `Φ_safety`, and
- windowed delta `δ_window` that can trigger halt when convergence worsens
  beyond an explicit epsilon.

The mechanism separates convergence gating from safety monitoring: halt
thresholds are evaluated against `V_convergence`, while `Φ_safety` remains an
informational safety measure. This limits feedback loops that could make slash
accumulation directly alter convergence gating.

## Technical effect

The mechanism provides a deterministic and replayable way to evaluate validator
stability. It can halt unsafe state progression when projected validator updates
produce excessive convergence degradation, while preserving fixed-point,
checked-arithmetic behavior across platforms.

## Implementation details

- `crates/consensus/src/lyapunov.rs` defines weights, bounded metric checks,
  convergence windows, delta computation, and evaluation output.
- `crates/consensus/src/transition.rs` evaluates a projected post-input state
  before committing validator updates.
- `crates/consensus/src/fixed_point.rs` supplies checked scaled arithmetic with
  floor division toward negative infinity.
- `docs/spec/01_consensus.md` specifies the consensus state space and stability
  measures.

## Alternative embodiments

- Different deterministic weights for validator divergence, conflict, and slash
  evidence selected at genesis.
- Multiple convergence windows for short-term and long-term stability.
- A read-only runtime monitor that reports `δ_window` without halting, while a
  stricter network profile enables the halt trigger.

## Failure cases prevented

- Validator updates that increase bounded divergence beyond the permitted
  convergence window.
- Platform-specific floating-point differences in stability scoring.
- Slash accumulator decreases or invalid metric bounds entering consensus state.
- Arithmetic overflow silently wrapping stability values.

## Candidate claim elements

1. A deterministic validator stability evaluator using fixed-point bounded
   metrics.
2. A windowed Lyapunov-style convergence comparison coupled to a state transition
   halt path.
3. A projected-state evaluation step performed before committing validator
   updates.
4. A separation between convergence halt gating and safety monitoring terms.
5. A monotone slash accumulator incorporated into a safety value without
   directly controlling the convergence halt threshold.

## Diagrams

- `../diagrams/state_transition_machine.mmd`

## Experimental evidence to attach

- Unit tests for convergence-window updates and fixed-point arithmetic.
- Golden replay traces that include stable and halt-triggering validator update
  sequences.
- Benchmarks measuring evaluation cost as validator count approaches the static
  maximum.
