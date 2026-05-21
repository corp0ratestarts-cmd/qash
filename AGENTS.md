# Repository Guidelines

## Project Structure & Module Organization

QASH is a Rust workspace with a strict Domain A/Domain B split. `crates/consensus/` is the deterministic `no_std` consensus core. `crates/pal/` is the Platform Abstraction Layer for hosted runtime, networking, attestation, and other nondeterministic services. `model/` contains the canonical executable model bridge to Coq. `proofs/` contains Coq theorem files and proof status. `docs/spec/`, `docs/adr/`, and `docs/traceability.md` document protocol requirements and audit mapping. Integration and replay vectors live under `crates/*/tests/` and `tests/vectors/`.

## Build, Test, and Development Commands

- `cargo build --workspace`: build all Rust workspace crates.
- `cargo test --workspace`: run the full native Rust test suite.
- `cargo test -p qash-consensus --no-default-features`: test the deterministic consensus core.
- `cargo test -p qash-pal --features std`: test hosted PAL functionality.
- `make -C proofs`: compile configured Coq proof targets when Coq is installed.
- `cargo deny check`: run supply-chain policy checks.
- `git diff --check`: catch whitespace errors before committing.

## Coding Style & Naming Conventions

Use Rust 2021 and standard `rustfmt` formatting. Keep Domain A code deterministic: no `unsafe`, no floats, no wall-clock, no entropy ingress, no nondeterministic iteration, and no `unwrap()`, `expect()`, `panic!()`, or `unreachable!()` in consensus-critical paths. Use explicit-width integer types for state and wire values. Prefer descriptive snake_case module, function, and test names.

## Testing Guidelines

Tests are Rust unit/integration tests plus Coq proof compilation. Replay and determinism tests should pin observable roots or transcripts, not implementation details. Name tests by behavior, for example `v1_2_sharded_corpus_matches_pinned` or `hosted_whole_protocol_sharded_replay_is_deterministic`. When touching consensus behavior, run the consensus tests and any affected vector tests; when touching PAL, run the `std` PAL suite.

## Commit & Pull Request Guidelines

Commit messages should be concise, present-tense, and usually conventional-style: `fix: update pal tests`, `docs: document causal fingerprint hash axiom`, or `feat(consensus): add replay gate`. Keep one logical change per commit. PRs must include a summary, exactly one type from the template, local verification, and linked issue context when applicable. Coq changes require zero `Admitted` outside `_wip/` and updates to proof status. Genesis constant changes require the explicit `[genesis-change-acknowledged]` token.

## Security & Architecture Notes

Domain B may observe clocks, transport, entropy, and attestation, but those observations must not directly influence Domain A state. All protocol-affecting inputs must be canonicalized and replayable.
