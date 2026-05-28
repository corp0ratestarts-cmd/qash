# Rust Bad Practices Scan

**Commit:** `7c1d41fd2447b0aedd507e32ad5e9208c16980cc`
**Timestamp:** 2026-05-27T07:04:01Z
**Domain A status:** ✅ PASS
**Domain B advisory count:** 29 finding(s)

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

## Domain A results (blocking)

- **Directory:** `crates/consensus/src`
- **Test code:** stripped via awk filter (from `check_domain_a_tripwires.sh`)

✅ No violations found.

## Domain B results (advisory)

- **Directories:** crates/pal/src crates/address/src model/src src
- **Test code:** stripped via awk filter

⚠️ **29 advisory finding(s) — triage required before genesis-lock:**

- `[unsafe] crates/pal/src/admission.rs:72:            unsafe { core::ptr::write_volatile(byte, 0) };`
- `[unwrap()] crates/pal/src/mvp.rs:108:        let version = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:110:        let epoch = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:188:        let version = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());`
- `[unwrap()] crates/pal/src/mvp.rs:190:        let epoch = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());`
- `[expect()] crates/pal/src/crypto/drbg.rs:93:    getrandom::getrandom(&mut buf).expect("OS entropy unavailable");`
- `[expect()] crates/pal/src/mvp_vault.rs:628:        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");`
- `[SystemTime/Instant/OsRng/getrandom] crates/pal/src/crypto/drbg.rs:93:    getrandom::getrandom(&mut buf).expect("OS entropy unavailable");`
- `[SystemTime/Instant/OsRng/getrandom] crates/pal/src/mvp_vault.rs:100:            getrandom::getrandom(&mut salt)`
- `[SystemTime/Instant/OsRng/getrandom] crates/pal/src/lib.rs:112:    use std::time::{SystemTime, UNIX_EPOCH};`
- `[SystemTime/Instant/OsRng/getrandom] crates/pal/src/lib.rs:542:            SystemTime::now()`
- `[std::fs::/std::net::/std::env::] crates/pal/src/mvp_vault.rs:15:use std::fs::{self, File, OpenOptions};`
- `[std::fs::/std::net::/std::env::] crates/pal/src/recovery_wal.rs:117:            let mut file = std::fs::OpenOptions::new()`
- `[std::fs::/std::net::/std::env::] crates/pal/src/recovery_wal.rs:132:        let mut file = std::fs::OpenOptions::new()`
- `[std::fs::/std::net::/std::env::] crates/pal/src/recovery_wal.rs:144:        let mut file = std::fs::File::open(&self.path).map_err(|_| RecoveryWalError::Io)?;`
- `[std::fs::/std::net::/std::env::] crates/pal/src/net/tcp_transport.rs:13:use std::net::{TcpListener, TcpStream, ToSocketAddrs};`
- `[std::fs::/std::net::/std::env::] crates/pal/src/net/tcp_transport.rs:109:    pub fn local_addr(&self) -> io::Result<std::net::SocketAddr> {`
- `[std::fs::/std::net::/std::env::] crates/pal/src/lib.rs:109:    use std::fs::{File, OpenOptions};`
- `[loop {}] crates/pal/src/mvp_vault.rs:594:    loop {`
- `[loop {}] crates/pal/src/recovery_wal.rs:152:        loop {`
- `[loop {}] crates/pal/src/lib.rs:652:        loop {`
- `[unwrap()] crates/address/src/lib.rs:67:    let l: [u8; 32] = payload[..32].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:68:    let r: [u8; 16] = payload[32..].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:81:    let d: [u8; 32] = payload[..32].try_into().unwrap();`
- `[unwrap()] crates/address/src/lib.rs:82:    let c: [u8; 16] = payload[32..].try_into().unwrap();`
- `[expect()] src/demo.rs:446:        write!(&mut out, "{byte:02x}").expect("write to String cannot fail");`
- `[SystemTime/Instant/OsRng/getrandom] src/demo.rs:422:    getrandom::getrandom(&mut out).map_err(|err| DemoCliError::Random(err.to_string()))?;`
- `[std::fs::/std::net::/std::env::] src/main.rs:10:    let args: Vec<String> = std::env::args().skip(1).collect();`
- `[std::fs::/std::net::/std::env::] src/bin/qash-demo.rs:5:    let args: Vec<String> = std::env::args().skip(1).collect();`

## Verdict

**PASS** — Domain A is clean. Domain B has 29 advisory finding(s) requiring triage.
