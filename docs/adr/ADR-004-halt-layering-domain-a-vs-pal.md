# ADR-004 — Absorbing halt layering: consensus vs PAL
**Status:** Accepted  
**Filed:** 2026-05-13  
**PDF anchor:** §2.3 (pp. 3–4) defines `trigger_absorbing_halt(..) -> !` with zeroize/scheduler/watchdog.

## Decision
- Domain A (`crates/consensus`): deterministically enters HALTED state and emits `HaltReason`.
- Domain B (`crates/pal`): performs zeroize/scheduler disable/watchdog reset and does not return.

## Acceptance criteria
- Unit tests: halt is absorbing/idempotent in consensus
- Integration test (hosted PAL): halt path terminates process (simulating `-> !`)
