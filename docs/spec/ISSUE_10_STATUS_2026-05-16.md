# Issue/PR #10 status check (2026-05-16)

This repository state (branch `work`, commit `973b008`) does **not** contain the implementation described in GitHub PR #10 (`feat: qash-model, qash-address, 7-family derive cascade, simulation binary`).

## Missing vs #10 description

- `crates/address/` crate is not present.
- `model/` crate source implementation is not present (`model/README.md` exists, but no Rust crate source tree).
- `crates/consensus/src/derive.rs` is not present.
- `docs/spec/11_handshake_protocol.md` is not present in this tree.
- `src/main.rs` currently prints params hash + encoding version only, and does not include simulation outputs described by #10.

## Strategic next steps

1. Decide whether PR #10 branch should be merged/rebased into `work`.
2. If not merging as-is, open a follow-up implementation PR that introduces:
   - `qash-address` crate
   - `qash-model` crate with deterministic scenarios/tests
   - `derive.rs` cascade and verification tests
   - simulation-capable `src/main.rs`
3. Add CI checks that assert these modules exist and compile in workspace to prevent silent drift.

## Notes

- This status note is a local repository check artifact for traceability.
