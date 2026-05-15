# Invention Disclosure: Deterministic Replay Isolation Architecture

## Problem statement

Replicated state machines can diverge when wall clocks, entropy, network order,
allocator behavior, hardware-specific execution, or platform-width types enter
the state transition path. Conventional testing can miss divergence because the
same binary and host are often used for development and replay.

## Prior-art limitations

Typical blockchain and consensus systems separate networking from consensus in
implementation, but the boundary is frequently an engineering convention rather
than a protocol-level conformance rule. That makes it difficult to prove that a
runtime replay on a different host observes the same transition relation.

## Novel mechanism

QASH defines a two-domain execution architecture:

1. **Domain A** contains the consensus transition function, canonical encoding,
   state roots, admissibility checks, deterministic arithmetic, and cryptographic
   operations over consensus data.
2. **Domain B** contains nondeterministic operational services such as wall
   clocks, networking, hardware attestation, logging, OS interaction, and
   acceleration.
3. Domain B may submit externally signed candidate inputs, but Domain A must
   verify admissibility before the input can affect consensus state.
4. Any Domain B value that influences Domain A transition semantics is a
   protocol conformance failure, even when tests pass.
5. Domain A outputs may be passed outward to Domain B, preserving a one-way
   observational boundary for state roots, halt signals, and replay validation.

## Technical effect

The architecture converts nondeterminism from an implicit runtime hazard into an
explicitly quarantined class of nonconforming flows. The expected technical
benefits are reproducible replay, cross-platform state-root equivalence, bounded
failure handling through absorbing halts, and clearer proof scope for formal
verification.

## Implementation details

- Protocol law for the domain partition is specified in
  `docs/spec/00_execution_model.md`.
- Domain A implementation is located in `crates/consensus/` and is `no_std`.
- Domain B implementation is located in `crates/pal/` and may contain hosted,
  hardware-specific, or unsafe operational code under audit.
- Arithmetic failures in Domain A map to explicit halt behavior rather than
  platform-dependent wrapping.
- Canonical state encoding and domain-tagged hashing are used to bind state
  observations to deterministic bytes.

## Alternative embodiments

- A statically linked embedded runtime in which Domain B is a minimal transport
  shim and Domain A is compiled without allocator support.
- A hosted server runtime in which Domain B performs networking and telemetry,
  while Domain A is invoked as a pure replay verifier.
- A formal model extraction pipeline in which the Domain A runtime must remain
  observationally equivalent to an executable proof-derived model.

## Failure cases prevented

- Clock-derived epoch values changing a state transition.
- Randomness or hardware attestation outcomes modifying consensus semantics.
- Hash-map iteration order influencing validator updates.
- Platform-width `usize` values affecting encoded state.
- Network arrival order bypassing admissibility checks.

## Candidate claim elements

1. A replicated state execution system partitioned into deterministic and
   nondeterministic execution domains.
2. A boundary rule set preventing nondeterministic-domain values from altering a
   deterministic state transition.
3. A replay verifier that accepts only admissible candidate inputs before state
   transition.
4. An absorbing halt path for arithmetic or decoding violations in the
   deterministic domain.
5. A domain-tagged state-root computation over canonical encodings of
   deterministic state.

## Diagrams

- `../diagrams/replay_isolation_sequence.mmd`
- `../diagrams/state_transition_machine.mmd`

## Experimental evidence to attach

- Golden replay traces from `crates/consensus/tests/golden_replay.rs`.
- Cross-ISA state-root comparisons stored under
  `../../artifacts/replay_equivalence/`.
- Boundary violation tests demonstrating rejection of clock, entropy, or
  trailing validator-slot contamination.
