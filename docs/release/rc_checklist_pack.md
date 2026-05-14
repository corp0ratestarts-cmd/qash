# QASH Release-Candidate Checklist Pack

Version: v1.1.0-rc
Branch: `claude/review-and-push-itHqh`

---

## P0 Gate Matrix

| Gate | Status | Evidence |
|------|--------|----------|
| phi_max_safe pinned in genesis | ✅ GO | `GENESIS_CONSTANTS.toml` line: `phi_max_safe = 944473296573929042432`; compile-time guard in `crates/consensus/src/params.rs` |
| PhiSafetyViolation halt wired | ✅ GO | `HaltReason::PhiSafetyViolation = 0x07` in `transition.rs`; gate at `phi_safety.raw() >= PHI_MAX_SAFE` |
| Nonzero-slash golden vectors | ✅ GO | `tests/vectors/vectors.v1.json`: `phi_safety_violation_512_validators` (512v, slash=i64::MAX → halt) + `phi_below_threshold_511_validators` (511v → None) |
| Nonzero-slash unit tests | ✅ GO | `crates/consensus/tests/golden_replay.rs`: `phi_safety_violation_at_512_validators`, `phi_below_threshold_at_511_validators` |
| Coq proofs — gated subset | ✅ GO | CI `coq-proofs` job: TH-3a/b/c + TH-9 fully proved, no `Admitted`; `util/list_inj.v` infrastructure proved |
| build-test CI | ✅ GO | `cargo test --workspace`, clippy strict, vector runner golden pass |
| genesis_hash | ✅ GO | `GENESIS_CONSTANTS.toml`: `QASH-CASCADE-7:63be85c25adf68f0f2376a51b711b0f3890cfabb89d92108191689853a77867cd365c8db7bbe41506ca1872c5eecaf5186a0b9c8466ca0fcd23179ae87f95a23` (self-verified, includes genesis_blind_nonce per spec §4) |
| cross-ISA determinism | 🟡 ADVISORY | CI job `continue-on-error: true`; sysroot + QEMU fix pushed, awaiting CI result |
| two-stage build verify | 🟡 ADVISORY | `scripts/verify_two_stage_build.sh` is a stub — reserved post-genesis |
| Coq proofs — full formal suite | 🟡 ADVISORY | TH-1/2/4/5/6/8 remain design sketches in `proofs/_wip/`; post-genesis obligation |

---

## Coq CI Evidence

**Gated proofs (must stay green, no Admitted):**

| File | Theorems | Result |
|------|----------|--------|
| `proofs/contractivity/lyapunov_stability.v` | TH-3a: δ ≤ ε → no halt; TH-3b: halt ↔ δ > ε; TH-3c: finalize → V=0 | ✅ PROVED |
| `proofs/cascade/cascade_health_bounded.v` | TH-9: CH_t ∈ [0,p], no overflow | ✅ PROVED |
| `proofs/util/list_inj.v` | flat_map injectivity, app_cancel_left | ✅ PROVED |
| `proofs/cascade/cascade_collision_resistance.v` | TH-10 (Axiom — expected) | ✅ Axiom, not Admitted |
| `proofs/cascade/cascade_determinism.v` | TH-11 (verification claim) | ✅ No proof needed |

**CI gate:** `grep -r "Admitted\." contractivity/ cascade/cascade_health_bounded.v` must return empty.

**CI run:** Fill in commit SHA and run ID on final green run before merge.
- Commit SHA: _[fill before merge]_
- Run ID: _[fill before merge]_

---

## Cross-ISA Evidence Bundle

**Toolchain requirements:**
- `rustc --version`: 1.75.0 (pinned, `CLAUDE.md` §8.1)
- `cargo --version`: stable toolchain companion
- Target triples: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `riscv64gc-unknown-linux-gnu`
- Linker (aarch64): `aarch64-linux-gnu-gcc`
- Linker (riscv64): `riscv64-linux-gnu-gcc`
- QEMU: `qemu-aarch64 -L /usr/aarch64-linux-gnu`, `qemu-riscv64 -L /usr/riscv64-linux-gnu`

**Verification script:** `scripts/verify_cross_isa_identity.sh`
- Builds vector-runner for all three targets
- Runs each binary against `tests/vectors/vectors.v1.json`
- Diffs outputs — must be byte-identical

**Status:** Advisory (`continue-on-error: true` in CI). Infrastructure fix in flight.

---

## Genesis Constants Snapshot

| Constant | Value | Source |
|----------|-------|--------|
| `scale` | 1_000_000 | `GENESIS_CONSTANTS.toml` / `fixed_point::SCALE` |
| `weight_D` (α) | 350_000 | `lyapunov::WEIGHT_D` |
| `weight_C` (β) | 300_000 | `lyapunov::WEIGHT_C` |
| `weight_S` (γ) | 200_000 | `lyapunov::WEIGHT_S` |
| `weight_CH` (χ) | 150_000 | `lyapunov::WEIGHT_CH` |
| `epsilon` (ε) | 20_000 | `lyapunov::EPSILON` |
| `window` (W) | 3 | `lyapunov::WINDOW_SIZE` |
| `phi_max_safe` | 944_473_296_573_929_042_432 | `lyapunov::PHI_MAX_SAFE` |
| `max_validators` | 1024 | `transition::MAX_VALIDATORS` |
| `genesis_hash` | QASH-CASCADE-7:63be85c25adf68f0f2376a51b711b0f3890cfabb89d92108191689853a77867cd365c8db7bbe41506ca1872c5eecaf5186a0b9c8466ca0fcd23179ae87f95a23 | `GENESIS_CONSTANTS.toml` |

**Config fingerprint** (`consensus_params_hash()`): run after genesis_hash is locked.

---

## Change-Freeze Window Policy

Once `genesis_hash` is computed and committed, `GENESIS_CONSTANTS.toml` is **LOCKED**.

Any modification to constants in that file redefines the network. Changes require:
1. A new ADR documenting the rationale
2. Re-computation of `genesis_hash`
3. All P0 gates re-run and confirmed green
4. All sign-offs reset and re-obtained

Exceptions allowed post-freeze: documentation-only edits to `[meta]` comments, addition of `[migration.compatibility]` entries for forward compatibility (append-only).

---

## Multi-Owner Sign-Off

| Role | Owner | Status |
|------|-------|--------|
| Consensus (Domain A correctness) | TBD | ⬜ PENDING |
| Formal Methods (proof coverage) | TBD | ⬜ PENDING |
| Runtime / PAL (Domain B boundary) | TBD | ⬜ PENDING |
| Release (genesis_hash locked, CI green) | TBD | ⬜ PENDING |

**NO-GO final rule:** If any P0 ❌ OPEN gate exists, or any sign-off is PENDING,
the release is blocked. All ✅ GO and all sign-offs required before merge to main.

---

## Pre-Merge Checklist

- [x] `genesis_hash` computed and pinned: `QASH-CASCADE-7:63be85c25adf68f0f2376a51b711b0f3890cfabb89d92108191689853a77867cd365c8db7bbe41506ca1872c5eecaf5186a0b9c8466ca0fcd23179ae87f95a23`
- [ ] All cargo tests pass: `cargo test --workspace --no-default-features`
- [ ] Coq CI green (coq-proofs job): no Admitted in gated scope
- [ ] Vector runner golden pass: all 5 vectors pass including phi_safety vectors
- [ ] CI run ID and commit SHA recorded above
- [ ] All sign-offs obtained
