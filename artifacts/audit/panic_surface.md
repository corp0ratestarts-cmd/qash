# Panic Surface Scan

**Commit:** `bd96d07441de0f2cc01e94895e38de7513e75ccb`
**Timestamp:** 2026-05-28T10:05:39Z
**Domain A status:** ✅ PASS
**Domain A debug assertion advisory count:** 0 finding(s)
**Domain B advisory count:** 22 finding(s)

## Patterns scanned (whitespace-tolerant)

- `unwrap()` — `unwrap[[:space:]]*\(`
- `expect()` — `expect[[:space:]]*\(`
- `panic!()` — `panic![[:space:]]*\(`
- `assert!()` — `(^|[^[:alnum:]_])assert![[:space:]]*\(`
- `assert_eq!()` — `(^|[^[:alnum:]_])assert_eq![[:space:]]*\(`
- `assert_ne!()` — `(^|[^[:alnum:]_])assert_ne![[:space:]]*\(`
- `lock().unwrap()` — `lock\(\)[[:space:]]*\.[[:space:]]*unwrap[[:space:]]*\(`
- `join().unwrap()` — `join\(\)[[:space:]]*\.[[:space:]]*unwrap[[:space:]]*\(`

## Debug assertion patterns (Domain A advisory only)

- `debug_assert!()` — `debug_assert![[:space:]]*\(`
- `debug_assert_eq!()` — `debug_assert_eq![[:space:]]*\(`
- `debug_assert_ne!()` — `debug_assert_ne![[:space:]]*\(`

## Test stripping

All scans strip `#[test]` functions and `#[cfg(test)]` modules via the
awk filter from `check_domain_a_tripwires.sh:34-55`. Comment lines
(`//`, `///`, `//!`) are also excluded.

## Domain A results (blocking)

- **Directory:** `crates/consensus/src`

✅ No blocking violations found.

## Domain A debug assertions (advisory)

✅ No debug assertions found.

## Domain B results (advisory)

- **Directories:** crates/pal/src crates/address/src model/src src

⚠️ **22 advisory finding(s) — triage required before genesis-lock:**

- `[unwrap()] crates/pal/src/mvp.rs:108:        let version = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:110:        let epoch = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:188:        let version = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:190:        let epoch = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());`
- `[expect()] crates/pal/src/crypto/drbg.rs:61:            .expect("infallible entropy source cannot fail")`
- `[expect()] crates/pal/src/crypto/drbg.rs:97:            .expect("infallible DRBG entropy source cannot fail")`
- `[expect()] crates/pal/src/crypto/drbg.rs:165:    try_os_entropy().expect("OS entropy unavailable")`
- `[expect()] crates/pal/src/mvp_vault.rs:760:        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");`
- `[assert!()] crates/pal/src/crypto/drbg.rs:56:        assert!(`
- `[assert_eq!()] crates/pal/src/privacy/public_transcript.rs:25:    assert_eq!(transcript.state_root.len(), ROOT_LEN);`
- `[assert_eq!()] crates/pal/src/privacy/public_transcript.rs:26:    assert_eq!(transcript.receipt_root.len(), ROOT_LEN);`
- `[assert_eq!()] crates/pal/src/privacy/public_transcript.rs:27:    assert_eq!(transcript.efb_root.len(), ROOT_LEN);`
- `[unwrap()] crates/address/src/lib.rs:67:    let l: [u8; 32] = payload[..32].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:68:    let r: [u8; 16] = payload[32..].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:81:    let d: [u8; 32] = payload[..32].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:82:    let c: [u8; 16] = payload[32..].try_into().unwrap();`
- `[assert!()] model/src/lib.rs:77:    assert!(`
- `[expect()] src/demo.rs:674:        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");`
- `[debug_assert!()] src/crypto/cascade_coq.rs:26:        debug_assert!(cascade_fail_count >= 0);`
- `[debug_assert!()] src/crypto/cascade_coq.rs:27:        debug_assert!(cascade_fail_count <= max_queries_per_epoch);`
- `[debug_assert!()] src/crypto/cascade_coq.rs:28:        debug_assert!(max_queries_per_epoch > 0);`
- `[debug_assert!()] src/crypto/cascade_coq.rs:30:        debug_assert!((0..=Self::P).contains(&value));`

## Verdict

**PASS** — Domain A blocking panic surface is clean. Domain A debug assertions and Domain B findings remain advisory triage items.
