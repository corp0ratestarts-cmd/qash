# Invention Disclosure: Cross-ISA Deterministic Reproducibility Enforcement

## Problem statement

A replicated protocol may pass local tests while producing different roots on a
different architecture, compiler target, optimization level, or endian-sensitive
encoding path. Cross-ISA divergence is especially dangerous because it may appear
only after independent operators deploy heterogeneous machines.

## Prior-art limitations

Many systems document deterministic serialization or provide golden tests, but
fewer define an enforcement pipeline that combines protocol-level arithmetic
rules, canonical byte encoding, deterministic hashing, no-platform-width state,
and two-stage build or cross-target replay checks.

## Novel mechanism

QASH treats cross-ISA equivalence as a protocol conformance requirement rather
than an optimization goal. The mechanism combines:

1. deterministic Domain A semantics,
2. explicit integer-width and fixed-point laws,
3. canonical little-endian encodings,
4. domain-tagged cryptographic hashing,
5. golden replay state-root comparison, and
6. build verification intended to detect target-dependent behavior.

## Technical effect

The expected effect is identical state evolution and state-root production for
admissible inputs across authorized execution environments. Divergence becomes a
conformance failure that can be localized through replay traces and hash
checkpoints.

## Implementation details

- Arithmetic and execution laws are specified in `docs/spec/00_execution_model.md`.
- Fixed-point encoding uses explicit byte widths in
  `crates/consensus/src/fixed_point.rs`.
- Canonical consensus encoding lives in `crates/consensus/src/encoding.rs`.
- Domain-tagged hashing is implemented in `crates/consensus/src/hash.rs`.
- Golden replay tests live in `crates/consensus/tests/golden_replay.rs`.
- Two-stage build verification is initiated by `scripts/verify_two_stage_build.sh`.

## Alternative embodiments

- A CI matrix comparing state roots from x86_64, aarch64, and RISC-V targets.
- A deterministic replay artifact bundle containing input vectors, encoded
  states, state roots, and compiler metadata.
- A runtime self-test mode that replays a fixed vector before joining a network.

## Failure cases prevented

- Endianness-dependent root computation.
- `usize` or allocator-order-dependent state changes.
- Compiler-target-dependent arithmetic overflow behavior.
- Floating-point rounding differences across processors.
- Hardware acceleration changing consensus-visible cryptographic bytes.

## Candidate claim elements

1. A cross-target replay verification process for a deterministic consensus
   state transition implementation.
2. Canonical fixed-width state encoding coupled with domain-tagged state hashing.
3. A build pipeline comparing deterministic outputs produced by independently
   built artifacts.
4. A replay artifact schema that binds inputs, encoded states, roots, target
   triples, and toolchain identifiers.
5. A conformance gate rejecting implementations whose replay roots differ across
   authorized targets.

## Diagrams

- `../diagrams/replay_isolation_sequence.mmd`

## Experimental evidence to attach

- Per-target replay logs under `../../artifacts/replay_equivalence/`.
- CI output showing matching state roots for all authorized targets.
- Negative tests demonstrating that intentional endian or arithmetic mutations
  change the replay root.
