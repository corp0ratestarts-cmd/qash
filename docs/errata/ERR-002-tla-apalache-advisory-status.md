# ERR-002: TLA+/Apalache formal verification is advisory, not a v1.0 genesis-lock gate

**Date:** 2026-06-01
**Status:** accepted
**Scope:** PDF §3.10.2 — TLA+ safety invariant; `docs/traceability.md` P0-7

---

## PDF quote

> **§3.10.2 (p. 32), Formal verification obligations (TH-7 CI gate):**
>
> "The TLA+ safety invariant (`proofs/safety/`) must be checked by Apalache model
> checking for all reachable state sequences up to length 100."

---

## Defect / ambiguity

The PDF text in §3.10.2 states a TLA+ model-checking obligation as if it were a CI
gate equivalent to TH-7. Prior versions of `docs/traceability.md` (pre-Wave 1)
quoted this as a shell command (`apalache-mc check --length=100 tla/QASHConsensus.tla`)
and marked P0-7 as a CI-required check, implying a `tla/` directory and an Apalache
binary are required in the CI environment.

The ambiguity is:
1. No `tla/QASHConsensus.tla` file exists in the repository.
2. Apalache is not installed in any CI workflow.
3. The PDF §3.10.2 places this obligation under "Formal verification obligations,"
   a section that groups **proof targets** (TH-1 through TH-8 and the Coq proof
   corpus), not CI enforcement gates.
4. The genesis-lock gate in PDF §3.11.4 enumerates TH-1, TH-2, TH-3a, TH-3b,
   TH-4, TH-5, TH-6, TH-7, and TH-8 as the required formal results. TLA+/Apalache
   is not separately enumerated as a gate.

---

## Resolution

TLA+/Apalache model checking is an **advisory / post-genesis** formal verification
target. It is not required for the v1.0 genesis-candidate lock.

The concrete resolution:

1. `docs/traceability.md` P0-7 is updated (Wave 1, PR #227) to read:
   > ⚠️ **ADVISORY** — TLA+/Apalache model checking is a post-genesis formal
   > verification target. PDF §3.10.2 cites it as a proof obligation under the
   > formal verification obligations section; it is not separately enumerated in
   > the genesis-lock gate (§3.11.4). No `tla/` sources exist yet; Apalache is
   > not in CI. Scheduled for Wave 2 post-v1.0.

2. `proofs/COVERAGE.md` does not list TLA+/Apalache as a CI-VERIFIED or PROVED
   property; it is captured in the open proof obligations as a post-v1.0 item.

3. The genesis-lock gate (PR #240) does not require TLA+/Apalache to pass.

---

## Impact statement

No genesis-lock dependency. The v1.0 safety properties are fully covered by the
existing Coq proof corpus (TH-1 through TH-8, RT-1 through RT-4, TX-0, TX-1).
TLA+/Apalache would provide an independent machine-checked verification of the
high-level state-machine invariants; adding it post-genesis is the recommended path.

---

## References

- PDF §3.10.2 (p. 32): Formal verification obligations
- PDF §3.11.4 (p. 35): Genesis-lock gate enumeration
- `docs/traceability.md` P0-7 (updated Wave 1, PR #227)
- `proofs/COVERAGE.md` — open proof obligations section
