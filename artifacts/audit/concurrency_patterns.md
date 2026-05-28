# Concurrency Pattern Audit

**Commit:** `eaa8f614842d47af2e4b7501cb77326a2681ef86`
**Timestamp:** 2026-05-28T10:50:42Z
**Status:** Advisory — exit 0 always. Findings require triage before genesis-lock.

## Pattern summary

| Pattern | Hit count |
|---------|-----------|
| `Mutex` | 3 |
| `RwLock` | 0 |
| `Arc<Mutex>` | 0 |
| `Atomic* types` | 5 |
| `Ordering::Relaxed` | 0 |
| `spawn()` | 2 |
| `.await` | 0 |
| `thread::sleep` | 0 |

## Lock-across-await candidates

✅ No lock-across-await patterns detected.

## Detailed findings by pattern

### `Mutex` (3 hits)

- `src/hardware/power_management.rs:7:use std::sync::Mutex;`
- `src/hardware/power_management.rs:41:    state: Mutex<PowerState>,`
- `src/hardware/power_management.rs:53:            state: Mutex::new(initial_state),`

### `RwLock` (0 hits)

_No hits._

### `Arc<Mutex>` (0 hits)

_No hits._

### `Atomic* types` (5 hits)

- `crates/pal/src/receipt.rs:56:pub enum AtomicShredError<VaultError, WalError> {`
- `crates/pal/src/receipt.rs:73:    /// Atomically complete local key erase/revocation and return durable evidence.`
- `crates/pal/src/receipt.rs:108:) -> Result<ShredCommitment, AtomicShredError<V::Error, W::Error>>`
- `crates/pal/src/receipt.rs:115:        .map_err(AtomicShredError::Vault)?;`
- `crates/pal/src/receipt.rs:117:        .map_err(AtomicShredError::EvidenceAppend)?;`

### `Ordering::Relaxed` (0 hits)

_No hits._

### `spawn()` (2 hits)

- `crates/pal/src/net/tcp_transport.rs:136:        let handle = std::thread::spawn(move || {`
- `crates/pal/src/net/tcp_transport.rs:168:        let handle = std::thread::spawn(move || {`

### `.await` (0 hits)

_No hits._

### `thread::sleep` (0 hits)

_No hits._

## Verdict

**Advisory only** — this scan always exits 0. All findings require triage
and a documented decision in `docs/audit/dependency_risk_register.md`
before genesis-lock. Lock-across-await candidates should be reviewed
against the `await_holding_lock` Clippy lint (see Phase 3).
