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
- Hosted PAL persisted input logs used for crash recovery.

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
| Crash truncation | Restart observes a partially written hosted input | Persist only complete canonical records; replay rejects malformed records |
| Attestation contamination | TPM quote bytes are mixed into state or entropy | Attestation remains Domain B evidence and is never read by the transition |
| Reset contamination | Watchdog or operator reset selects a new transition branch | Restart begins by replaying the canonical log from genesis |

## Domain B → Domain A boundary

The boundary is an allow-list, not a deny-list. Hosted runtime facilities may
observe nondeterministic Domain B facts such as wall-clock time, packet arrival
order, local transport retries, attestation quotes, process crashes, and
watchdog/reset requests. Those facts are not parameters of Domain A.

The only hosted ingress allowed to affect Domain A is a canonical input record:

1. `epoch`: the expected Domain A epoch before applying the record.
2. `updates`: one fixed slot per active validator, either absent (`None`) or a
   fixed-width integer encoding of divergence, conflict, and monotone slash
   accumulator values.
3. `raw_txs`: byte strings passed as transaction candidates to the deterministic
   Domain A transaction pipeline.

A hosted implementation must satisfy the following rules before a Domain A
transition can occur:

- **Normalize first:** transport frames and local host metadata are converted to
  a canonical input record outside Domain A. Arrival time, arrival order, source
  socket, retry count, attestation material, and reset state are not part of the
  record.
- **Validate at the transition:** the record is converted to `EpochInput` only
  when its epoch and validator-slot count match the current `EpochState`; the
  consensus transition then performs metric, halt, and transaction admissibility
  checks.
- **Persist only accepted input:** the hosted PAL appends a record to durable
  storage only after `advance_epoch` accepts it. Rejected or malformed Domain B
  observations are not durable Domain A history.
- **Replay from genesis:** crash recovery reconstructs state by replaying the
  persisted canonical records from a supplied genesis state. Wall-clock time,
  network queues, attestation quotes, and reset flags are intentionally ignored
  during replay.
- **Reject non-canonical history:** malformed log headers, record tags,
  non-zero absent-update payloads, invalid padding, trailing bytes, and truncated
  records are log errors rather than alternate transition semantics.

## Minimal hosted runtime milestone

The milestone for `crates/pal` is deliberately small and Domain-B-contained:

| Capability | Milestone behavior | Domain A influence |
| --- | --- | --- |
| Deterministic ingress boundary | `CanonicalInput` carries epoch, validator-slot updates, and raw transaction bytes | Only after conversion to `EpochInput` and `advance_epoch` validation |
| Persistent state storage | Append-only canonical input log with a fixed file header and framed records | Replay supplies the same accepted input sequence from genesis |
| Network transport | Hosted queues capture ingress/egress byte frames | No state mutation; frames must be normalized into canonical inputs separately |
| Time | Hosted wall-clock counter is available as Domain B data | Never read by transition or replay |
| Attestation | Hosted quote bytes are available as Domain B evidence | Never read by transition or replay |
| Halt/reset | Hosted reset request/abort belongs to PAL control flow | Recovery uses canonical replay, not reset timing |
| Crash recovery | Restart opens the log and replays complete records from genesis | Identical persisted logs must produce identical state roots |

## Security invariants

1. Domain B values do not influence Domain A transition semantics.
2. Domain A arithmetic is checked and deterministic.
3. Encoding is canonical and independent of host architecture.
4. Halt conditions are deterministic and replay-visible.
5. Cross-target replay artifacts must produce identical roots for the same
   admissible input sequence.
6. Hosted crash recovery must derive state exclusively from genesis plus the
   persisted canonical input log.

## Evidence tasks

- Add negative tests for clock, entropy, and trailing-slot contamination.
- Archive replay equivalence artifacts for authorized targets.
- Add mutation tests for endian and arithmetic rule violations.
- Add CI checks that reject forbidden Domain A APIs.
- Extend hosted PAL log fuzzing to cover corrupt/truncated records.
