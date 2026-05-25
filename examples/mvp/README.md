# QASH MVP Fixture Pack

Deterministic reference fixtures for the offline incident-receipt commit demonstrator.

## Purpose

Pilot operators can use these fixtures to verify that their local build produces
byte-identical public commitment records and commitment roots to the reference.
This guards against silent divergence introduced by toolchain or dependency updates.

## Running the fixture generator

```bash
cargo run --example mvp_fixtures
```

The output is deterministic across all authorised ISAs (x86-64, AArch64, RISC-V)
given the same pinned Rust toolchain. Compare the printed `commitment_root` with
the root produced by the full demo flow:

```bash
bash scripts/run_mvp_demo.sh --clean
```

## Fixture inputs (fixed seed, no randomness)

| Label | Epoch | Body |
|-------|-------|------|
| alpha-incident | 1 | `synthetic alpha incident body` |
| beta-incident  | 2 | `synthetic beta incident body`  |
| gamma-incident | 3 | `synthetic gamma incident body` |

Nonces and disclosure commitments are derived from fixed rotation patterns
(see `examples/mvp_fixtures.rs` for the exact derivation).

## Claim boundary

These fixtures are for local Domain B demonstrator verification only.
They are not production incident data and do not represent any real event.
All allowed and blocked claims are governed by `docs/mvp/claims_register.md`.
