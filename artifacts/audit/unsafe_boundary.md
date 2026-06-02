# Unsafe Boundary Audit

**Commit:** `698887404a4b0f0cf5e5f83d3f6285bc9e4b7f5c`  
**Timestamp:** 2026-06-01T23:36:00Z  
**Domain A status:** ✅ PASS
**Domain B missing SAFETY comment:** 0 advisory finding(s)
**Domain B with SAFETY comment:** 8 compliant site(s)

## Policy

**Domain A:** `qash-consensus` has `#![forbid(unsafe_code)]`. Any `unsafe` hit
exits 1 unconditionally. SAFETY comments and exception entries do not override
— Domain A forbids unsafe absolutely.

**Domain B:** Any `unsafe` block or function without a preceding `// SAFETY:`
comment (within 5 lines) AND without an entry in `docs/audit/unsafe_exceptions.md`
→ advisory finding requiring triage before genesis-lock.

**unsafe detection pattern** (precise — skips `forbid`/`deny` attribute lines):
```
unsafe\s*(\{|fn\s|impl\s|trait\s|extern\s)
```

## Domain A results (blocking)

- **Directory:** `crates/consensus/src`

✅ No unsafe found — consistent with `#![forbid(unsafe_code)]`.

## Domain B results (advisory)

### Compliant unsafe sites (have // SAFETY: comment or exception entry)

- ✅ `crates/pal/src/zk/fib_air.rs:39:         let (prefix, shorts, suffix) = unsafe { self.align_to::<FibRow<F>>() };`
- ✅ `crates/pal/src/zk/fib_air.rs:101:     let (prefix, rows, suffix) = unsafe { values.align_to_mut::<FibRow<QashVal>>() };`
- ✅ `crates/pal/src/hardening.rs:21: unsafe impl Send for RowhammerGuard {}`
- ✅ `crates/pal/src/hardening.rs:38:     pub unsafe fn register_region(&mut self, ptr: *mut u8, len: usize) {`
- ✅ `crates/pal/src/hardening.rs:48:     pub unsafe fn refresh_all(&self) {`
- ✅ `crates/pal/src/hardening.rs:66: unsafe fn softtrr_refresh_region(ptr: *mut u8, len: usize) {`
- ✅ `crates/pal/src/hardening.rs:79: unsafe fn softtrr_refresh_region(_ptr: *mut u8, _len: usize) {`
- ✅ `crates/pal/src/admission.rs:75:             unsafe { core::ptr::write_volatile(byte, 0) };`


## cargo geiger count summary (advisory)

```
error: no such command: `geiger`

help: view all installed commands with `cargo --list`
help: find a package to install `geiger` with `cargo search cargo-geiger`
(cargo geiger failed or not installed — advisory only)
```

## Verdict

**PASS** — Domain A is clean. Domain B has 0 advisory finding(s) requiring triage.
