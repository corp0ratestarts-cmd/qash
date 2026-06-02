# Panic Surface Scan

**Commit:** `698887404a4b0f0cf5e5f83d3f6285bc9e4b7f5c`  
**Timestamp:** 2026-06-01T23:35:56Z  
**Domain A status:** ✅ PASS
**Domain A debug assertion advisory count:** 0 finding(s)
**Domain B advisory count:** 57 finding(s)

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

⚠️ **57 advisory finding(s) — triage required before genesis-lock:**

- `[unwrap()] crates/pal/src/mvp.rs:108:        let version = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:110:        let epoch = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:188:        let version = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:190:        let epoch = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());`
- `[unwrap()] crates/pal/src/crypto/tls.rs:109:        s.push(char::from_digit(u32::from(*b) >> 4, 16).unwrap());`
- `[unwrap()] crates/pal/src/crypto/tls.rs:110:        s.push(char::from_digit(u32::from(*b) & 0x0f, 16).unwrap());`
- `[unwrap()] crates/pal/src/clone/wipe.rs:106:        let session_id: [u8; 32] = bytes[9..41].try_into().unwrap();`
- `[unwrap()] crates/pal/src/clone/wipe.rs:107:        let epoch = u64::from_le_bytes(bytes[41..49].try_into().unwrap());`
- `[unwrap()] crates/pal/src/clone/wipe.rs:108:        let issuer_pk: [u8; 32] = bytes[49..81].try_into().unwrap();`
- `[unwrap()] crates/pal/src/clone/wipe.rs:109:        let signature: [u8; 64] = bytes[81..145].try_into().unwrap();`
- `[unwrap()] crates/pal/src/clone/transport/frame.rs:123:        let epoch = u64::from_le_bytes(bytes[1..9].try_into().unwrap());`
- `[unwrap()] crates/pal/src/clone/transport/frame.rs:124:        let chunk_idx = u16::from_le_bytes(bytes[9..11].try_into().unwrap());`
- `[unwrap()] crates/pal/src/clone/transport/frame.rs:125:        let chunk_total = u16::from_le_bytes(bytes[11..13].try_into().unwrap());`
- `[unwrap()] crates/pal/src/clone/transport/frame.rs:126:        let compressed_len = u16::from_le_bytes(bytes[13..15].try_into().unwrap()) as usize;`
- `[expect()] crates/pal/src/crypto/drbg.rs:61:            .expect("infallible entropy source cannot fail")`
- `[expect()] crates/pal/src/crypto/drbg.rs:97:            .expect("infallible DRBG entropy source cannot fail")`
- `[expect()] crates/pal/src/crypto/drbg.rs:165:    try_os_entropy().expect("OS entropy unavailable")`
- `[expect()] crates/pal/src/crypto/dual_hash.rs:44:    let ctx_len = u32::try_from(context.len()).expect("context exceeds u32::MAX");`
- `[expect()] crates/pal/src/crypto/dual_hash.rs:45:    let salt_len = u32::try_from(salt.len()).expect("salt exceeds u32::MAX");`
- `[expect()] crates/pal/src/crypto/dual_hash.rs:46:    let data_len = u64::try_from(data.len()).expect("data exceeds u64::MAX");`
- `[expect()] crates/pal/src/crypto/dual_hash.rs:61:    let ctx_len = u32::try_from(context.len()).expect("context exceeds u32::MAX");`
- `[expect()] crates/pal/src/crypto/dual_hash.rs:62:    let salt_len = u32::try_from(salt.len()).expect("salt exceeds u32::MAX");`
- `[expect()] crates/pal/src/crypto/dual_hash.rs:63:    let data_len = u64::try_from(data.len()).expect("data exceeds u64::MAX");`
- `[expect()] crates/pal/src/mvp_vault.rs:760:        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");`
- `[expect()] crates/pal/src/receipt.rs:127:        .expect("viewing key is always 32 bytes");`
- `[expect()] crates/pal/src/receipt.rs:131:        .expect("ChaCha20-Poly1305 encrypt must not fail with valid key");`
- `[assert!()] crates/pal/src/crypto/drbg.rs:56:        assert!(`
- `[assert!()] crates/pal/src/zk/fib_air.rs:97:    assert!(n.is_power_of_two(), "trace height must be a power of two");`
- `[assert!()] crates/pal/src/zk/fib_air.rs:102:    assert!(prefix.is_empty());`
- `[assert!()] crates/pal/src/zk/fib_air.rs:103:    assert!(suffix.is_empty());`
- `[assert_eq!()] crates/pal/src/privacy/public_transcript.rs:25:    assert_eq!(transcript.state_root.len(), ROOT_LEN);`
- `[assert_eq!()] crates/pal/src/privacy/public_transcript.rs:26:    assert_eq!(transcript.receipt_root.len(), ROOT_LEN);`
- `[assert_eq!()] crates/pal/src/privacy/public_transcript.rs:27:    assert_eq!(transcript.efb_root.len(), ROOT_LEN);`
- `[debug_assert!()] crates/pal/src/zk/fib_air.rs:40:        debug_assert!(prefix.is_empty());`
- `[debug_assert!()] crates/pal/src/zk/fib_air.rs:41:        debug_assert!(suffix.is_empty());`
- `[debug_assert_eq!()] crates/pal/src/zk/fib_air.rs:36:        debug_assert_eq!(self.len(), NUM_FIBONACCI_COLS);`
- `[debug_assert_eq!()] crates/pal/src/zk/fib_air.rs:42:        debug_assert_eq!(shorts.len(), 1);`
- `[unwrap()] crates/address/src/lib.rs:67:    let l: [u8; 32] = payload[..32].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:68:    let r: [u8; 16] = payload[32..].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:81:    let d: [u8; 32] = payload[..32].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:82:    let c: [u8; 16] = payload[32..].try_into().unwrap();`
- `[assert!()] model/src/lib.rs:77:    assert!(`
- `[expect()] src/demo.rs:674:        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");`
- `[expect()] src/genesis_preimage.rs:34:            let text = std::str::from_utf8(&raw).expect("GENESIS_CONSTANTS.toml must be UTF-8");`
- `[expect()] src/bin/genesis_hash.rs:14:    let repo_root_str = env::args().nth(1).expect("usage: genesis-hash <repo-root>");`
- `[expect()] src/bin/genesis_cert.rs:22:    let repo_root_str = env::args().nth(1).expect("usage: genesis-cert <repo-root>");`
- `[expect()] src/bin/genesis_cert.rs:56:            .expect("cannot read rust-toolchain.toml"),`
- `[expect()] src/bin/genesis_cert.rs:59:        sha3_256_hex(&std::fs::read(repo_root.join("Cargo.lock")).expect("cannot read Cargo.lock"));`
- `[expect()] src/bin/genesis_cert.rs:147:        .expect("Argon2id computation failed");`
- `[expect()] src/bin/genesis_cert.rs:158:    .expect("invalid Argon2id params")`
- `[expect()] src/bin/genesis_cert.rs:174:        .expect("cannot run rustc --version --verbose");`
- `[panic!()] src/genesis_preimage.rs:20:        .unwrap_or_else(|e| panic!("cannot read {}: {}", manifest_path.display(), e));`
- `[panic!()] src/genesis_preimage.rs:31:        let raw = fs::read(&path).unwrap_or_else(|e| panic!("cannot read artifact {}: {}", rel, e));`
- `[debug_assert!()] src/crypto/cascade_coq.rs:42:        debug_assert!(cascade_fail_count >= 0);`
- `[debug_assert!()] src/crypto/cascade_coq.rs:43:        debug_assert!(cascade_fail_count <= max_queries_per_epoch);`
- `[debug_assert!()] src/crypto/cascade_coq.rs:44:        debug_assert!(max_queries_per_epoch > 0);`
- `[debug_assert!()] src/crypto/cascade_coq.rs:46:        debug_assert!((0..=P).contains(&value));`

## Verdict

**PASS** — Domain A blocking panic surface is clean. Domain A debug assertions and Domain B findings remain advisory triage items.
