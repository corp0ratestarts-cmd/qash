# Internal Audit Report — 2026-05-17

## Scope
- Repository-level sanity test run (`cargo test -q`).
- Consensus crate comprehensive test run (`cargo test -p qash-consensus -q`).
- Static warning review from compiler output.

## Findings

### 1) Test coverage signal at workspace root is weak (Informational)
Running `cargo test -q` at workspace root executed zero tests in each discovered package phase, which can hide issues if contributors assume it provides broad coverage.

**Impact**: Potential false confidence from root-level test command.

**Recommendation**: Prefer crate-specific test commands (for example `cargo test -p qash-consensus`) in CI and contributor docs, or wire the root package/workspace to execute meaningful aggregate tests.

### 2) Test code warnings in consensus test suite (Low)
The consensus test suite compiles and passes but emits warnings:
- unused assignment in `crates/consensus/tests/golden_replay.rs`
- unused import and unused variable/assignment in `crates/consensus/tests/adversarial.rs`

**Impact**: Low immediate risk, but warnings can mask newly introduced defects and reduce signal quality.

**Recommendation**: Clean up unused imports/assignments/variables or enforce warning-free tests in CI (`-D warnings` for test builds where feasible).

## Positive Signals
- `qash-consensus` test suite passed end-to-end with no failing tests.
- No panic/failure surfaced during this audit pass.

## Follow-up
- Treat this as a lightweight internal audit. For higher-assurance review, add:
  - `cargo clippy --all-targets --all-features -D warnings`
  - dependency/license/security checks (e.g., `cargo deny` / advisory DB)
  - fuzz target execution cadence and coverage thresholds.
