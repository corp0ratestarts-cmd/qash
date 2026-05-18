# ADR-005: Rust Toolchain Version

- **Status:** accepted, revised 2026-05-18
- **PDF anchor:** §8.1, p. 23
- **Traceability rows:** P0-8

## Verbatim PDF text

```text
Rust 1.75.0 (pinned)
```

## Decision

Pin exactly Rust 1.95.0 in `rust-toolchain.toml` and treat that file as the
single source of truth for CI jobs that compile, test, lint, fuzz, or install
Rust-based tooling.

## Rationale

The PDF-selected Rust 1.75.0 pin is no longer viable for the current repository
state because the workspace lockfile is Cargo lockfile format v4, which is not
accepted by the Rust 1.75.0 cargo release. Keeping the repository on Rust 1.75.0
would therefore require either regenerating/downgrading the lockfile format or
blocking locked dependency resolution in modern Cargo workflows.

Rust 1.95.0 is now the accepted reproducibility pin. It is an exact stable
release available in the local audit environment, supports the current lockfile,
and has been locally verified for the workspace build path.

## Acceptance criteria

- `rust-toolchain.toml` exists and pins `channel = "1.95.0"`.
- CI installs the pinned toolchain from `rust-toolchain.toml` instead of using a
  floating `stable` action/ref.
- CI prints `rustc --version --verbose` and fails if `release:` does not match
  the pinned `rust-toolchain.toml` channel.
- Cross-ISA vectors must pass under the pinned toolchain before genesis lock.

## Future change process

Any move away from Rust 1.95.0 requires a new ADR or a revision of this ADR with
compatibility, reproducibility, and vector evidence.
