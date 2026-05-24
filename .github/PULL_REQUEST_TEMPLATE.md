## Summary

<!-- One paragraph: what does this PR do and why? -->

## Type

<!-- Check exactly one: -->
- [ ] Proof change (Coq `.v` file)
- [ ] Consensus code — Domain A (`crates/consensus/`, `no_std`, checked arithmetic)
- [ ] Spec document update (`docs/spec/`)
- [ ] CI / process / tooling
- [ ] **Genesis constants change** (`GENESIS_CONSTANTS.toml`) ← requires `[genesis-change-acknowledged]` token below
- [ ] Other

## Checklist

### Deduplication
- [ ] I searched open **and recently closed** PRs for overlapping work before opening this
- [ ] This PR does not duplicate an existing open or merged PR

### Human review
- [ ] A human (not only an AI assistant) has read and understood the changes in this PR
- [ ] The changes have been tested or verified locally, not only in CI

### Documentation hygiene
- [ ] This PR does not add raw chat transcripts, prompt dumps, or uncurated review exports
- [ ] Protocol/spec content is published in canonical docs (`docs/spec/`, `docs/adr/`, `docs/traceability.md`, or an accepted erratum), not as an ad hoc root file

### Domain A constraints *(skip if no `crates/consensus/` changes)*
- [ ] No `unsafe`, no `f32`/`f64`, no `usize`/`isize` in state struct fields or wire arithmetic
- [ ] All arithmetic uses checked operations (`checked_add`, `checked_mul`, etc.) or is `lia`-provable in Coq
- [ ] No new `unwrap()` / `expect()` / `panic!()` / `unreachable!()` — overflow routes to `Halt::absorbing_reset()`

### Performance claims *(skip if this PR makes no performance claim)*
- [ ] Benchmark artifacts are archived under `artifacts/benchmarks/`
- [ ] Any Phase 2-R runtime optimization preserves consensus bytes exactly and cites `docs/adr/ADR-006-runtime-optimization-track.md`

### Coq proofs *(skip if no `.v` file changes)*
- [ ] `grep -r "^Admitted" proofs/` (excluding `_wip/`) returns **zero results**
- [ ] Any new proof files are listed in the CI Tier 2 compile list in `.github/workflows/ci.yml`
- [ ] Any new proof files have a corresponding row in `proofs/STATUS.md`

### Genesis constants *(skip if `GENESIS_CONSTANTS.toml` not changed)*

> Genesis constant changes define a new network. They must be reviewed by a
> human, not merged automatically. Add the token below to acknowledge.

- [ ] This change is intentional and has been reviewed by a human
- [ ] `genesis_hash` has been recomputed over the updated document set

**Acknowledgment token** (paste as-is to unblock the genesis-guard CI check):

```
[genesis-change-acknowledged]
```

## Integration review evidence (required for slice-critical changes)

- Evidence manifest path: `artifacts/evidence/<timestamp>-<commit>/manifest.txt`
- Evidence manifest link (URL to this PR branch file):
- I confirm the manifest includes `Captured (UTC)`, commit SHA fields, and a
  `## Slice Command Statuses` table with `PASS` for all required commands.

## Verification

<!-- How was this tested? Paste relevant command output, CI links, or test results. -->
