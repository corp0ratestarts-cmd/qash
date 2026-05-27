# Pre-Genesis Full-Repo Audit Plan

**Status:** Active — pre-genesis gate for Track 11 / genesis-lock. Synced with
PR #201 scanner behavior on 2026-05-27.
**Trigger:** Run before any genesis-candidate release. Also runs weekly on `main` and on every PR touching `.rs`, `docs/`, `scripts/`, or workflows.

---

## Purpose

Lint, Clippy, and the existing CI jobs (build, test, tripwires, proofs, supply-chain) cover many mechanical issues but do not reliably catch:

- Protocol-invariant violations
- Domain A / Domain B contamination (platform, RTOS, GPU, HSM, TPM, TEE imports in consensus)
- Semantic infinite loops without obvious termination
- Lock-order deadlocks and async holding patterns
- Unsafe soundness gaps in Domain B code
- Claim overreach in documentation (compliance, certification, platform support)
- Dependency triage gaps (OSV/RustSec findings without a documented decision)

This audit plan defines a layered gate that distinguishes **blocking failures** (must fix before genesis-lock), **advisory findings** (record, triage, justify, or schedule), and **documented exceptions** (owner-accepted with sign-off).

---

## Gating model

| Category | Definition | Required before genesis-lock |
|----------|-----------|------------------------------|
| **Blocking** | Script exits 1; finding is unambiguous and project-controlled | Yes — must be clean |
| **Advisory** | Script continues on error; finding requires triage | Yes — triage entry required; fix optional |
| **Exception** | Known deviation; documented in exception register with owner sign-off | Yes — register entry required |

---

## Phase 1 — File inventory (`audit_file_inventory.sh`)

**Status:** Blocking (completion check — script must finish and produce output).

Classifies every file tracked by `git ls-files` into:

| Class | Path prefix |
|-------|------------|
| `domain-a` | `crates/consensus/src/` |
| `domain-b` | `crates/pal/src/`, `crates/address/src/`, `model/src/` |
| `binary` | `src/` |
| `proofs` | `proofs/` |
| `ci-workflow` | `.github/workflows/` |
| `scripts` | `scripts/` |
| `docs` | `docs/`, `spec/`, `tla/`, `patents/` |
| `tests` | `tests/`, `fuzz/` |
| `artifacts` | `artifacts/` |
| `config` | Root config files |
| `other` | Everything else |

**Output:** `artifacts/audit/file_inventory.{json,md}`

---

## Phase 2 — Rust bad-practice scan (`audit_rust_bad_practices.sh`)

**Status:** Blocking (Domain A tier); Advisory (Domain B tier).

**`unsafe` detection pattern** (precise — skips `forbid`/`deny` attribute lines and comments):
```
unsafe\s*(\{|fn\s|impl\s|trait\s|extern\s)
```

**Domain A (blocking) patterns** — production code only (test blocks stripped via awk):
```
unsafe\s*(\{|fn\s|impl\s|trait\s|extern\s)
unwrap\s*\(       expect\s*\(       panic!\s*\(
unreachable!\s*\( todo!\s*\(        unimplemented!\s*\(
get_unchecked\s*\( from_utf8_unchecked\s*\(
MaybeUninit       mem::zeroed       mem::transmute
static\s+mut\s    Ordering::Relaxed thread::sleep
SystemTime|Instant|OsRng|thread_rng|getrandom
std::fs::|std::net::|std::env::
tokio::            loop\s*\{        while\s+true    as\s+\*
```

**Domain B tier** — same patterns, advisory counts only.

**Output:** `artifacts/audit/rust_bad_practices.md`

---

## Phase 3 — Strict Clippy advisory profile (`run_clippy_strict_audit.sh`)

**Status:** Advisory.

Runs `cargo clippy --workspace --all-targets --no-default-features` with pedantic, nursery, and QASH-specific lints:
- `indexing_slicing`, `integer_arithmetic`, `cast_possible_truncation`, `cast_sign_loss`
- `await_holding_lock`, `mutex_atomic`, `mutex_integer`

Output captured to `artifacts/audit/strict_clippy.txt`. Exit 0 always — promotes individual lints to blocking only after triage.

---

## Phase 4 — Unsafe boundary audit (`audit_unsafe_boundary.sh`)

**Status:** Blocking (Domain A = unconditional exit 1); Advisory (Domain B = missing SAFETY comment or exception entry).

**Domain A policy:** `qash-consensus` has `#![forbid(unsafe_code)]`. Any `unsafe` hit exits 1 unconditionally. SAFETY comments and exception entries do not override — Domain A forbids unsafe absolutely.

**Domain B policy:** Any `unsafe` block or function without a preceding `// SAFETY:` comment (within 5 lines) AND without an entry in `docs/audit/unsafe_exceptions.md` → advisory finding requiring triage.

Also runs `cargo geiger --all-features` for a count summary (advisory).

**Output:** `artifacts/audit/unsafe_boundary.md`

---

## Phase 5 — Liveness loop scan (`audit_liveness_loops.sh`)

**Status:** Blocking (Domain A WARN); Advisory (Domain B / scripts).

Finds `loop\s*{`, `while\s+true`, `while\s+let` and checks the next 20 lines for:
```
break | return | recv\s*\( | sleep\s*\( | yield | \.await | Halt:: | // INTENTIONAL_LOOP:
```

**SAFE** — has an obvious termination or explicit `// INTENTIONAL_LOOP:` comment.  
**WARN** — no obvious termination; Domain A WARN → exit 1.

**Output:** `artifacts/audit/liveness_loops.md`

---

## Phase 6 — Panic surface scan (`audit_panic_surface.sh`)

**Status:** Blocking (Domain A); Advisory (Domain B).

Scans production code (test blocks stripped) for:
```
unwrap\s*\(  expect\s*\(  panic!\s*\(
assert!\s*\( assert_eq!\s*\( assert_ne!\s*\(
lock\(\)\.unwrap\s*\(  join\(\)\.unwrap\s*\(
```

Domain A → exit 1. Domain B → count/warn.

**Output:** `artifacts/audit/panic_surface.md`

---

## Phase 7 — Concurrency pattern audit (`audit_concurrency_patterns.sh`)

**Status:** Advisory.

Scans for: `Mutex`, `RwLock`, `Arc<Mutex`, `Atomic[A-Za-z]*`, `Ordering::Relaxed`, `spawn\s*\(`, `\.await`, `thread::sleep`. Flags lock-across-await patterns. Exit 0 always.

**Output:** `artifacts/audit/concurrency_patterns.md`

---

## Phase 9 — Claim boundary scan (`audit_claim_boundary.sh`)

**Status:** Blocking.

Scans all `.md`/`.toml`/`.txt` files (excluding `docs/mvp/claims_register.md`, `docs/audit/`, `docs/platforms/`, `docs/release/`). The `docs/funding/` and `docs/compliance/` directories **are** scanned — grant-facing and compliance-facing docs are exactly where overclaims are dangerous.

**Allowlist marker:** `<!-- claim-boundary-allow: <reason> -->` suppresses that line and the immediately following line only.

**Compliance / certification overclaim patterns** (case-insensitive): the scanner
blocks live claims for compliance certification, validation, authorization, production
readiness, regulated financial-infrastructure claims, and other phrases listed in
`scripts/audit_claim_boundary.sh`.

**Forbidden platform overclaims** (outside `docs/platforms/`): broad claims such as
support for all platforms or unrestricted platform/runtime support, plus profile-specific
support claims for MUSA, CUDA, ROCm, HSM, TPM, smartcard, TEE, or full RTOS support
before evidence exists. Narrow Tier A wording such as replay running on the three
authorized ISAs is not treated as a universal platform-support claim.

**Suppression policy:** clearly negative uses and explicit blocked/prohibited/avoid
example sections are not treated as live claims. The allowlist marker remains available
for one-off suppressions only.

**Output:** `artifacts/audit/claim_boundary.md`

---

## Phase 10 — Domain A / Domain B full boundary scan (`audit_domain_boundary_full.sh`)

**Status:** Blocking.

Extends the existing `check_domain_a_tripwires.sh`. Scans `crates/consensus/src/` for:

**Standard Domain B imports:**
```
qash_pal::|qash_address::|use std::net|use std::fs|use std::env
std::time::|SystemTime|Instant|OsRng|getrandom|rand::
serde_json|log::|tracing::|tokio::|async\s+fn|\.await
```

**Platform/accelerator/hardware contamination:**
```
itron::|freertos|zephyr|rtems|vxworks|qnx
cuda::|rocm::|musa::|opencl::|vulkan::|metal::|onedal::
tpm::|pkcs11::|javacard::|sgx::|trustzone::
```

**Output:** `artifacts/audit/domain_boundary_full.md`

---

## Phase 12 — Consolidated audit report (`build_pre_genesis_audit_report.sh`)

**Status:** Runs on `workflow_dispatch`, `push: main`, and weekly schedule only.

Reads all `artifacts/audit/*.md` and emits:
- `artifacts/audit/pre_genesis_full_repo_audit.md`
- `artifacts/audit/pre_genesis_full_repo_audit.json`

**Fields:** commit SHA, timestamp, file inventory by domain, unsafe/panic/unwrap counts, unclassified loop count, claim violations, domain boundary violations, platform matrix coverage, dependency finding count, proof status, open exception count, blocking pass/fail verdict.

---

## CI workflow split

See `.github/workflows/pre-genesis-full-repo-audit.yml`.

**Blocking jobs (every PR trigger):**
`claim-boundary`, `domain-boundary-full`, `rust-bad-practices`, `panic-surface`, `unsafe-boundary`, `liveness-loops`

**Full audit jobs (workflow_dispatch / push: main / schedule only):**
`file-inventory`, `build-audit-report`

**Advisory jobs (continue-on-error: true, every trigger):**
`strict-clippy`, `concurrency-patterns`, `miri-advisory`

---

## Negative test protocol

Before promoting any blocking script to CI, verify:

1. **unwrap test** — add `unwrap()` to production code in `crates/consensus/src/` → `audit_rust_bad_practices.sh` exits 1.
2. **overclaim test** — add "production-ready" to a top-level `.md` → `audit_claim_boundary.sh` exits 1.
3. **allowlist test** — add `<!-- claim-boundary-allow: test -->` before the overclaim → that line and the following line only are suppressed.
4. **blocked-example test** — add a forbidden phrase under an explicit blocked/prohibited/avoid examples section → `audit_claim_boundary.sh` suppresses it as documentation of a non-claim.
5. **unsafe false-positive test** — confirm `#![forbid(unsafe_code)]` does NOT trigger `audit_unsafe_boundary.sh`.
6. **platform contamination test** — add `use tokio::time;` to `crates/consensus/src/` → `audit_domain_boundary_full.sh` exits 1.

---

## Dependency risk register

Advisory findings from `cargo audit`, OSV scan, and `cargo deny` must be triaged into `docs/audit/dependency_risk_register.md` before genesis-lock. Each entry records: crate, version, CVE/advisory, direct/transitive, reachable in QASH, domain (A/B/tests/tooling), exploitability, fix available, decision, owner sign-off.

---

## Open exceptions register

See `docs/audit/unsafe_exceptions.md` for unsafe code exceptions. Each exception must have: crate, file, line range, justification, owner sign-off.
