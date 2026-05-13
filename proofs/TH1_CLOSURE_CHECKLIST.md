# TH-1 Closure Checklist (Focused Sprint)

Scope: close TH-1 (Encoding injectivity) first, then unblock TH-2 and TH-8.

## 1) Normalize theorem scaffolding

- [ ] Promote `_wip/encode_injectivity.v.draft` into a compilable `.v` target.
- [ ] Replace invalid pseudo-syntax (`apply ... by X by Y`) with valid Coq tactics.
- [ ] Eliminate scope mixing (`Z` vs `nat`) by fixing explicit coercions and section locals.

## 2) Minimize theorem kernel

- [ ] Isolate the exact encoding function definition used by consensus.
- [ ] Define auxiliary lemmas needed for injectivity as separately compilable units.
- [ ] Keep assumptions explicit and minimal; avoid introducing global axioms.

## 3) Compile and harden

- [ ] `coqc` succeeds locally for TH-1 target.
- [ ] Search the proof dependency chain to ensure no `Admitted` remains.
- [ ] Add a short proof note mapping each helper lemma to its role in TH-1.

## 4) Gate + status update

- [ ] Add/adjust CI step to execute `coqc` on TH-1 target.
- [ ] Update `proofs/STATUS.md` TH-1 row only after CI pass.
- [ ] If TH-1 closes, immediately advance TH-2 row from blocked to in-progress.

## Definition of done

TH-1 is considered CLOSED only when all "Proof done gate" criteria in
`proofs/STATUS.md` are satisfied.
