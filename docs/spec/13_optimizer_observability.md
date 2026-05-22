# QASH Optimizer Observability and Traffic-Shaping Profile
## `docs/spec/13_optimizer_observability.md` — Protocol Version 1.3 Draft

> **Status:** Derived engineering specification recovered from PR #93 critique
> follow-through. This is a Domain B policy/spec boundary document; it does not
> authorize Domain A behavior changes.

---

## §13.1 — Scope

This document defines how QASH implements traffic-shape privacy and runtime
optimizer behavior without violating Domain A determinism and purity.

Primary goal:

- hide workload-correlated traffic shape from passive observers
- preserve Domain A invariants and replay determinism
- avoid introducing cross-layer side channels

---

## §13.2 — Constant-Rate Emissions as Profile-Bounded Bands

QASH does not use a literal fixed-cadence packet metronome. Instead, it uses
**profile-bounded traffic-shaping bands** with bounded jitter.

Normative profile concept:

```
TrafficShapingBand = {
  band_id: u16,
  envelope_pps_min: u32,
  envelope_pps_max: u32,
  jitter_percent_max: u8,
  epoch_switch_policy: EpochBoundOnly
}
```

Rules:

- The visible envelope must remain within the configured band.
- Jitter must be cryptographically generated in Domain B and bounded by the
  configured `jitter_percent_max`.
- Band switches are allowed only at epoch boundaries.
- Mid-epoch switching based on private workload is forbidden.

Rationale:

- flat envelope shape hides micro-bursts from cross-shard receipts or sync load
- jitter removes cadence fingerprints that a strict metronome would leak
- epoch-bound switching prevents event-level traffic correlation

---

## §13.3 — Domain A Quarantine for Cover Traffic

Cover traffic must live entirely in Domain B.

Forbidden:

- generating dummy Domain A envelopes for padding
- feeding cover/padding artifacts into `PublicTranscript`
- modifying Domain A state, roots, or Lyapunov terms via padding logic

Required:

- padding occurs at Domain B transport/frame/batch level only
- Domain A receives only admissible protocol-affecting inputs
- Domain B padding must not affect Domain A replay outputs

---

## §13.4 — Cross-Layer Side-Channel Prohibition

Domain B profile changes must not be directly triggered by Domain A health
signals (for example `cascade_health` degradation).

Forbidden coupling examples:

- "if cascade health falls, switch immediately to high-throughput profile"
- "if halt-risk rises, disable shaping to flush queues"

Admissible control:

- blind deterministic schedule gates (for example fixed epoch modulo policy)
- deployment-profile policy that is independent of live consensus health values

---

## §13.5 — Profile Monotonicity (Ratchet Rule)

A `DomainBProfile` may improve performance knobs but must not reduce privacy,
masking, traffic-shaping, logging integrity, or crypto-hardening below the
profile baseline unless the deployment profile explicitly allows it.

Normative invariant:

```
profile_next >= profile_baseline on all privacy/safety axes
```

Where monotonic axes include at minimum:

- traffic-shaping floor
- masking order/family floor
- logging/audit integrity floor
- cryptographic hardening floor

---

## §13.6 — Future Implementation and CI Gates

Before this profile can be treated as implemented:

1. Add CI/static checks ensuring Domain B cover-traffic logic cannot call into
   consensus-state mutation paths.
2. Add deterministic tests proving band-switches happen only on epoch boundaries.
3. Add side-channel guard tests proving profile changes are not keyed directly to
   Domain A health signals.
4. Add monotonicity guards in `DomainBProfile` APIs so optimizer updates cannot
   silently downgrade privacy floors.

These are future implementation gates, not yet-complete claims.


---

## §13.7 — Queue/Backpressure Safety Envelope

Traffic-shaping must avoid two failure modes:

1. privacy collapse (unshaped burst leakage), and
2. liveness collapse (unbounded queue growth under sustained load).

Therefore each deployment profile must define a bounded shaping envelope with:

- per-band steady-state egress envelope (`envelope_pps_min/max`)
- maximum admissible queue depth for each transport class
- deterministic overload policy (drop/defer/retry) that remains Domain B-only

Overload handling must not trigger Domain A semantic shortcuts, synthetic
consensus envelopes, or transcript-visible dummy artifacts.

---

## §13.8 — Epoch-Bound Profile Schedule

Profile/band transitions are schedule-constrained:

- decision points occur only on epoch boundaries
- mid-epoch emergency profile jumps are inadmissible unless the deployment
  profile explicitly defines a safety exception class
- any exception class must be auditable and must not depend on private
  transaction-content features

This keeps profile transitions weakly predictable at the policy level while
preventing direct correlation of transitions to instantaneous private workload.

---

## §13.9 — Implementation Mapping (Planned)

Planned landing surfaces for this spec family:

- `crates/pal/` transport shaping and profile-control modules
- PAL tests validating epoch-bound switching and no Domain A mutation coupling
- release checklist assertions for Domain A/B cover-traffic separation
- CI/static checks that fail if padding code paths touch consensus mutation APIs

These mappings are planning targets and do not imply implementation-complete
status.
