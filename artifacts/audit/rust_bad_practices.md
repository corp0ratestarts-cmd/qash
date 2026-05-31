# Rust Bad Practices Scan

**Commit:** `ea6f8944be161343c202dd1dd1cbd5067df1a04d`  
**Timestamp:** 2026-05-31T00:46:49Z  
**Domain A status:** ✅ PASS
**Domain B advisory count:** 64 finding(s)

## Patterns scanned

**unsafe detection** (precise — skips `forbid`/`deny` attribute lines and comments):
```
unsafe\s*(\{|fn\s|impl\s|trait\s|extern\s)
```

**Additional patterns** (whitespace-tolerant):

- `unwrap()`
- `expect()`
- `panic!()`
- `unreachable!()`
- `todo!()`
- `unimplemented!()`
- `get_unchecked()`
- `from_utf8_unchecked()`
- `MaybeUninit`
- `mem::zeroed`
- `mem::transmute`
- `static mut`
- `Ordering::Relaxed`
- `thread::sleep`
- `SystemTime/Instant/OsRng/getrandom`
- `std::fs::/std::net::/std::env::`
- `tokio::`
- `loop {}`
- `while true`
- `as *ptr cast`
- `f32/f64 float (Domain A)`
- `HashMap (Domain A — use BTreeMap)`

## Domain A results (blocking)

- **Directory:** `crates/consensus/src`
- **Test code:** stripped via awk filter (from `check_domain_a_tripwires.sh`)

✅ No violations found.

## Domain B results (advisory)

- **Directories:** crates/pal/src crates/address/src model/src src
- **Test code:** stripped via awk filter

⚠️ **64 advisory finding(s) — triage required before genesis-lock:**

- `[unsafe] crates/pal/src/zk/fib_air.rs:39:        let (prefix, shorts, suffix) = unsafe { self.align_to::<FibRow<F>>() };`
- `[unsafe] crates/pal/src/zk/fib_air.rs:101:    let (prefix, rows, suffix) = unsafe { values.align_to_mut::<FibRow<QashVal>>() };`
- `[unsafe] crates/pal/src/hardening.rs:21:unsafe impl Send for RowhammerGuard {}`
- `[unsafe] crates/pal/src/hardening.rs:38:    pub unsafe fn register_region(&mut self, ptr: *mut u8, len: usize) {`
- `[unsafe] crates/pal/src/hardening.rs:48:    pub unsafe fn refresh_all(&self) {`
- `[unsafe] crates/pal/src/hardening.rs:66:unsafe fn softtrr_refresh_region(ptr: *mut u8, len: usize) {`
- `[unsafe] crates/pal/src/hardening.rs:79:unsafe fn softtrr_refresh_region(_ptr: *mut u8, _len: usize) {`
- `[unsafe] crates/pal/src/admission.rs:75:            unsafe { core::ptr::write_volatile(byte, 0) };`
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
- `[expect()] crates/pal/src/mvp_vault.rs:760:        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");`
- `[SystemTime/Instant/OsRng/getrandom] crates/pal/src/crypto/drbg.rs:158:    getrandom::getrandom(&mut buf).map_err(|_| DrbgError::EntropyUnavailable)?;`
- `[SystemTime/Instant/OsRng/getrandom] crates/pal/src/lib.rs:142:    use std::time::{SystemTime, UNIX_EPOCH};`
- `[SystemTime/Instant/OsRng/getrandom] crates/pal/src/lib.rs:614:            SystemTime::now()`
- `[SystemTime/Instant/OsRng/getrandom] crates/pal/src/mvp_vault.rs:112:            getrandom::getrandom(&mut salt)`
- `[std::fs::/std::net::/std::env::] crates/pal/src/net/tcp_transport.rs:13:use std::net::{TcpListener, TcpStream, ToSocketAddrs};`
- `[std::fs::/std::net::/std::env::] crates/pal/src/net/tcp_transport.rs:109:    pub fn local_addr(&self) -> io::Result<std::net::SocketAddr> {`
- `[std::fs::/std::net::/std::env::] crates/pal/src/recovery_wal.rs:117:            let mut file = std::fs::OpenOptions::new()`
- `[std::fs::/std::net::/std::env::] crates/pal/src/recovery_wal.rs:132:        let mut file = std::fs::OpenOptions::new()`
- `[std::fs::/std::net::/std::env::] crates/pal/src/recovery_wal.rs:144:        let mut file = std::fs::File::open(&self.path).map_err(|_| RecoveryWalError::Io)?;`
- `[std::fs::/std::net::/std::env::] crates/pal/src/lib.rs:139:    use std::fs::{File, OpenOptions};`
- `[std::fs::/std::net::/std::env::] crates/pal/src/mvp_vault.rs:15:use std::fs::{self, File, OpenOptions};`
- `[loop {}] crates/pal/src/recovery_wal.rs:152:        loop {`
- `[loop {}] crates/pal/src/lib.rs:569:            loop {`
- `[loop {}] crates/pal/src/lib.rs:724:        loop {`
- `[loop {}] crates/pal/src/mvp_vault.rs:726:    loop {`
- `[as *ptr cast] crates/pal/src/hardening.rs:71:        core::arch::x86_64::_mm_clflush(p as *const u8);`
- `[unwrap()] crates/address/src/lib.rs:67:    let l: [u8; 32] = payload[..32].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:68:    let r: [u8; 16] = payload[32..].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:81:    let d: [u8; 32] = payload[..32].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:82:    let c: [u8; 16] = payload[32..].try_into().unwrap();`
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
- `[SystemTime/Instant/OsRng/getrandom] src/demo.rs:650:    getrandom::getrandom(&mut out).map_err(|err| DemoCliError::Random(err.to_string()))?;`
- `[std::fs::/std::net::/std::env::] src/main.rs:10:    let args: Vec<String> = std::env::args().skip(1).collect();`
- `[std::fs::/std::net::/std::env::] src/hardware/platform.rs:44:        Self::from_arch(std::env::consts::ARCH)`
- `[std::fs::/std::net::/std::env::] src/bin/qash-demo.rs:5:    let args: Vec<String> = std::env::args().skip(1).collect();`
- `[std::fs::/std::net::/std::env::] src/bin/genesis_cert.rs:55:        &std::fs::read(repo_root.join("rust-toolchain.toml"))`
- `[std::fs::/std::net::/std::env::] src/bin/genesis_cert.rs:59:        sha3_256_hex(&std::fs::read(repo_root.join("Cargo.lock")).expect("cannot read Cargo.lock"));`
- `[loop {}] src/hardware/acceleration.rs:180:    loop {`

## Verdict

**PASS** — Domain A is clean. Domain B has 64 advisory finding(s) requiring triage.
