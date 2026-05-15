# Replay Equivalence Artifacts

This directory stores reproducibility evidence for deterministic replay and
cross-ISA state-root equivalence.

## Artifact naming

Use the following pattern:

```text
<date>-<scenario>-<target-triple>-<toolchain>.replay.md
```

Example:

```text
2026-05-15-golden-epoch-x86_64-unknown-linux-gnu-rust-1.XX.replay.md
```

## Required fields

Each replay artifact should include:

- Scenario name.
- Git commit hash.
- Spec hash, when genesis locking is active.
- Rust toolchain and target triple.
- Build profile and optimization flags.
- Input vector hash and, when small enough, canonical input bytes.
- Pre-state root.
- Post-state root or absorbing halt reason.
- Encoded state hash checkpoints.
- Command used to reproduce the run.
- Whether the result matches all other authorized targets.

## Minimal template

```text
# Replay Artifact: <scenario>

- Commit:
- Toolchain:
- Target triple:
- Command:
- Input vector hash:
- Pre-state root:
- Post-state root:
- Halt reason, if any:
- Matching target artifacts:
- Notes:
```
