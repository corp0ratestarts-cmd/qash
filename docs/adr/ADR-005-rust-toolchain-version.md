# ADR-005: Rust Toolchain Version

- **Status:** accepted
- **PDF anchor:** §8.1, p. 23
- **Traceability rows:** P0-8

## Verbatim PDF text

```text
Rust 1.75.0 (pinned)
```

## Decision

Pin exactly Rust 1.75.0 in `rust-toolchain.toml`.

## Rationale

The PDF is explicit and no accepted erratum changes this requirement. Deferring
the pin would keep P0-8 open without adding design clarity.

## Acceptance criteria

- `rust-toolchain.toml` exists and pins `channel = "1.75.0"`.
- CI uses the pinned toolchain instead of floating `stable`.
- Cross-ISA vectors must pass under the pinned toolchain before genesis lock.

## Future change process

Any move away from Rust 1.75.0 requires a new ADR or a revision of this ADR with
compatibility, reproducibility, and vector evidence.
