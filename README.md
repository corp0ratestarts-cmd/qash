# QASH Protocol

![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)
![Cross-Platform Determinism](https://github.com/<OWNER>/qash/actions/workflows/platform-determinism.yml/badge.svg)
![QASH CI](https://github.com/<OWNER>/qash/actions/workflows/ci.yml/badge.svg)

---

## What QASH Is

QASH is a **deterministic replicated transition calculus** with post-quantum
cryptographic anchoring and formally machine-checkable safety properties.

The central architectural invariant is:

> **Identical replay produces identical state.**
>
> All subsystems are subordinate to this guarantee.

QASH is **not**:
- A probabilistic consensus chain
- A Nakamoto-style longest-chain system
- A validator lottery or leader election system
- A speculative execution VM
- A conventional blockchain node

QASH **is**:
- A deterministic replicated state machine
- A replay-verifiable execution environment
- A formally constrained transition system whose safety properties
  are intended to be machine-provable from explicit axioms
- A protocol where serialization is ontology: two states that encode
  identically are the same state

---

## Repository Structure

```
spec/pdf/           ← Normative PDF source of truth (v1.0, checked in before genesis lock)
docs/traceability.md ← PDF requirement → code → test → proof contract
docs/errata/         ← Normative corrections/clarifications to the PDF
docs/adr/            ← Engineering decisions and PDF-silent gap definitions
docs/spec/           ← Pre-existing derived engineering specs pending mirror migration
  00_execution_model.md   Deterministic execution substrate
  01_consensus.md         State space, encoding, transition function, stability
  07_hash_cascade.md      Astronomical depth-7 cascade spec (v1.1)
  09_migration_v1.0_to_v1.1.md  Migration guide and compatibility window

proofs/             ← Formal theorems (Coq)
  contractivity/
    lyapunov_stability.v  TH-3a/TH-3b/TH-3c foundation proofs
    tx_perturbation_0.v   TX-0 §A8 Form A proof obligation
  util/
    list_inj.v            fixed-width list/encoding support lemmas
  _wip/
    encode_injectivity.v.draft  TH-1/TH-2 draft, not genesis-lock evidence
    absorbing_halt.v.draft      TH-4/TH-5/TH-6/TH-8 draft, not CI-gated

model/              ← Canonical executable semantics (extracted from proofs)
  README.md               Model contract and extraction notes

crates/
  consensus/        ← no_std consensus core (Domain A)
  pal/              ← Platform Abstraction Layer (Domain B)

src/                ← Hosted binary entrypoint
GENESIS_CONSTANTS.toml   Immutable genesis parameters (not yet locked)
```

> **Runtime status: thin scaffold — the hosted binary is a CLI demo only. PAL traits
> are wired but the Host implementation returns zeroes/no-ops. This is not a
> deployable node.**

```
```

The relationship between layers:

```
spec/pdf/     = normative source of truth      (what the protocol INTENDS)
docs/traceability.md = audit contract          (what is mapped to code/tests/proofs)
docs/errata/  = explicit PDF corrections       (what changes/clarifies the PDF)
docs/adr/     = engineering decisions          (how PDF gaps are filled)
proofs/       = formal guarantees             (what is PROVED about it)
model/        = canonical executable model    (what it COMPUTES, extracted from proofs)
crates/       = optimized implementation      (what is DEPLOYED)
```

The runtime (`crates/`) must be observationally equivalent to the model
for all admissible inputs. This equivalence is a future formal proof target.

The prior `docs/spec/` documents remain useful engineering specs, but the
repository now resolves authority through the PDF-first governance model in
`docs/traceability.md`: PDF quote → erratum/ADR if needed → code → test/vector
→ proof.

---

## Theorem Status

| ID | Name | Class | Status |
|----|------|-------|--------|
| TH-1 | Encoding injectivity | Formal theorem | ✅ FORMAL — `proofs/contractivity/encode_injectivity.v` |
| TH-2 | Encoding totality | Formal theorem | ✅ FORMAL — `proofs/contractivity/encode_injectivity.v` |
| TH-3 | Convergence decrease / halt gate | Formal theorem | ✅ FORMAL — `proofs/contractivity/lyapunov_stability.v` |
| TX-0 §A8 | No-op perturbation bound | Formal theorem | ✅ FORMAL — `proofs/contractivity/tx_perturbation_0.v` |
| TH-4 | Φ_safety monotonicity | Formal theorem | ✅ FORMAL — `proofs/safety/absorbing_halt.v` |
| TH-5 | Φ_safety boundedness | Formal theorem | ✅ FORMAL — `proofs/safety/absorbing_halt.v` |
| TH-6 | Halt correctness | Formal theorem | ✅ FORMAL — `proofs/safety/absorbing_halt.v` |
| TH-7 | Replay invariance RT-1 | Verification claim | ✅ CI-VERIFIED — identical state roots on x86_64, aarch64, riscv64gc (QEMU user-static) |
| TH-8 | Succession soundness | Formal theorem | ✅ FORMAL — `proofs/safety/absorbing_halt.v` + `proofs/integration/th8_composition.v` |

Genesis lock gate:
- TH-1 through TH-6, TH-8: **FORMAL** (Coq compiles; no `Admitted` beyond AX-1/AX-2/AX-3)
- TH-7: CI-verified on x86_64, aarch64, and riscv64gc (QEMU user-static; identical state roots)
- Archived drafts in `proofs/_wip/` are superseded — not lock evidence

---

## Foundational Axioms

All guarantees reduce to three axioms. Everything above them is deductively certain.

```
AX-1  Authorized ISAs implement two's complement arithmetic correctly
AX-2  Pinned Rust toolchain produces correct code for authorized ISAs
AX-3  Active consensus hash suite is collision-resistant  (cryptographic assumption, not theorem)
```

---

## `GENESIS_CONSTANTS.toml` Is Immutable

Once locked, `GENESIS_CONSTANTS.toml` cannot be modified.
Any change requires a new network. There are no protocol upgrades, governance
votes, or emergency patches. This is a design property, not a limitation.

---

## Contributing

QASH implementation discipline is closer to **kernel development** or
**avionics software** than conventional blockchain development.

### The non-negotiable rules for Domain A (consensus core)

```
FORBIDDEN in crates/consensus/ and anything it calls:
  - f32, f64, or any floating-point type
  - HashMap or HashSet (use BTreeMap, BTreeSet)
  - panic!(), unwrap(), expect()  — use explicit match + absorbing_reset()
  - unsafe blocks
  - std::time (wall clock)
  - OS randomness
  - nondeterministic iteration order
  - usize or isize  (use explicit u32/u64/i64/i128)
  - Rust default / on signed integers for protocol arithmetic  (use div_euclid)
  - Any heap allocation without statically-bounded size
```

### Domain B (PAL) rules

```
PERMITTED in crates/pal/:
  - unsafe under formal audit and review
  - SIMD and hardware acceleration
  - OS networking, clocks, entropy
  - Dynamic allocation

FORBIDDEN in crates/pal/:
  - Any Domain B value influencing Domain A state transitions
  - Clock or entropy inputs to the consensus execution path
```

### Every PR must

- Pass `cargo test --no-default-features` (consensus core)
- Pass `cargo build -p qash-pal --features std`
- Pass cross-ISA determinism check (automated in CI)
- Not introduce `Admitted` to any proof file without an explicit
  tracking issue and mathematical justification

### Contribution philosophy

> Nothing gets coded until it has a corresponding definition in the spec.
> No transaction type enters the protocol unless its effect on δ_window
> has a formal proof obligation filed.

New features that cannot be proved to preserve the convergence invariant
will not be merged, regardless of utility. This is the cost of formal
replay guarantees. It is also the source of QASH's long-term value.

---

## Spec Version Binding

The spec version is content-addressed:

```
spec_hash = SHA3-256(00_execution_model.md ∥ 01_consensus.md)
```

This hash is recorded in `GENESIS_CONSTANTS.toml` at genesis lock time.
Any runtime must declare the spec hash it implements. Mismatches are
flagged by CI as non-conforming, regardless of test passage.

---

## Patent Evidence Pack

The repository now includes a patent-support evidence structure for technical
review with qualified counsel. These materials are not legal advice; they
organize implementation-specific evidence around candidate invention families:

- deterministic replay isolation architecture,
- Lyapunov-based validator stability evaluation,
- cross-ISA deterministic reproducibility enforcement,
- prior-art differentiation working notes,
- claim-support traceability,
- replay and benchmark artifact templates,
- nondeterminism threat modeling, and
- architecture decision records.

Start at `patents/README.md`. Replay evidence should be archived under
`artifacts/replay_equivalence/`, and technical-effect measurements should be
archived under `artifacts/benchmarks/`.

---

*QASH is licensed GPL-3.0-or-later.*
*`GENESIS_CONSTANTS.toml` will be immutable after genesis lock.*
*Modifying it requires a new network.*
