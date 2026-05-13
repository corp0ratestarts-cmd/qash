# QASH Adversarial Model (Scoped)
## `docs/spec/04_adversarial_model.md` — Protocol Version 1.0

> **Status:** Normative adversarial scope for safety/liveness reasoning.
> This document constrains theorem obligations and simulation coverage.

---

## §A0 — Scope and Partition Alignment

This adversarial model applies to the same execution partition defined in
`00_execution_model.md §E0`.

- **Domain A (deterministic consensus):** theorem-bearing computation.
- **Domain B (transport/operations):** nondeterministic environment.

Consensus-state determinism claims are strictly Domain-A claims and do **not**
assume reliable transport, stable clocks, or fair scheduling.

---

## §A1 — Scenario Classes

### A1.1 Network partitions (Domain B)

Adversary may split honest validators into disjoint communication components for
an unbounded-but-finite duration.

**Assumptions:**
- Messages can be delayed or dropped across partition boundaries.
- Domain A transition semantics remain unchanged.

**Non-goals:**
- No guarantee of progress during active partition.
- No guarantee of minimal recovery time after heal.

### A1.2 Message reorderings / duplication (Domain B)

Adversary may reorder, duplicate, and delay delivery of signed external inputs.

**Assumptions:**
- Domain A admissibility checks reject malformed/invalid inputs.
- Replay and state evolution are defined only over canonical admissible input sets.

**Non-goals:**
- No FIFO transport guarantee.
- No exactly-once delivery guarantee.

### A1.3 Byzantine input construction (boundary adversary)

Adversary may generate syntactically valid but semantically conflicting signed
inputs, including equivocation attempts and slash-triggering evidence.

**Assumptions:**
- Cryptographic assumptions of configured primitives hold.
- Domain A verification is complete and deterministic.

**Non-goals:**
- No prevention claim for attempted Byzantine behavior.
- Only detection, containment, and deterministic accounting are claimed.

---

## §A2 — Safety vs Liveness Separation

### Safety claims (must hold under all A1 scenarios)

1. **Deterministic replay:** equal genesis + equal canonical admissible inputs
   imply equal post-state and equal `state_root`.
2. **No invalid transition acceptance:** malformed or inadmissible input does not
   produce an admissible next state.
3. **Slash accounting determinism:** slash evidence yields deterministic,
   bounded updates per protocol rules.

### Liveness claims (conditional)

Liveness is claimed only under explicit environmental conditions (eventual
message delivery, eventual partition healing, and sufficient active honest
validators). Liveness is not implied by safety nor by deterministic replay.

---

## §A3 — Theorem Obligations by Scenario Class

Each scenario class must have explicit obligations in the proof plan.

- **TH-A1 (Partition Safety):** safety invariants are preserved across any
  partition schedule and subsequent merge of admissible inputs.
- **TH-A2 (Reordering Safety):** permutation/duplication of transport delivery
  does not alter Domain-A outcome once canonical admissible input set is fixed.
- **TH-A3 (Byzantine Boundary Safety):** Byzantine inputs are either rejected or
  deterministically incorporated (e.g., slash evidence) without violating
  admissibility invariants.
- **TH-A4 (Conditional Liveness):** eventual progress under stated fairness and
  synchrony assumptions only.

Each theorem entry MUST include:
- exact assumptions,
- exact safety/liveness classification,
- explicit non-goals,
- mapping to executable simulation scenarios.

---

## §A4 — Simulation Hooks and CI Progression

Scenario-based simulation hooks are required for each A1 class:

1. **Phase 1 (non-blocking in CI):** run adversarial scenario suites with
   `continue-on-error` to collect flakiness/perf baselines.
2. **Phase 2 (blocking in CI):** promote each suite to required once stability
   criteria are met (documented pass-rate and runtime budget).

Promotion criteria and status tracking are maintained in `proofs/STATUS.md`.
