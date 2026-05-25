# MVP Baseline Review — Post-Merge Audit

**Scope:** Local Domain B offline incident receipt commit demonstrator only.  
**Date:** 2026-05-25  
**Triggered by:** PR #151 (MVP package merge)

## Audit scope

Four areas inspected:

1. `src/demo.rs` — Domain A contamination and private body leakage in CLI output
2. `crates/pal/src/mvp_vault.rs` — public export path, WAL privacy, fixture patterns
3. `crates/pal/src/mvp.rs` — public export encoding, body field exclusion
4. `docs/mvp/claims_register.md` — claim boundary accuracy vs. implementation

## Findings

### `src/demo.rs` — CLEAN

- No Domain A imports; all vault calls go through `qash_pal::mvp_vault` (Domain B).
- No private body text in any `println!` / `eprintln!` output.
- Body is accepted as a CLI argument and passed directly to `vault.issue_receipt()` — never printed.
- Help text carries an explicit claim-boundary disclaimer.

### `crates/pal/src/mvp_vault.rs` — CLEAN

- `export_public_commitments()` serializes only `record.public_export.encode()` — body bytes absent.
- `import_public_commitments()` deserializes only `TxMvpReceiptCommitPublicExport` (140-byte commitment-only records).
- Disclosure export writes to a private file under `DISCLOSURE_DIR`; it does not emit to stdout.
- Test fixture byte arrays are trivially derived (rotate_left/rotate_right/NOT) with no embedded cryptographic secrets.

### `crates/pal/src/mvp.rs` — CLEAN

- `TxMvpReceiptCommitPublicExport::encode()` serializes exactly 140 bytes:
  `version(4) + epoch(8) + tx_commitment(32) + nonce_commitment(32) + payload_commitment(32) + disclosure_key_commitment(32)`.
- No body field exists in the public export struct.
- An existing test explicitly validates that raw nonce and domain tag are excluded from public export.

### `docs/mvp/claims_register.md` — CLEAN

All allowed claims verified against implementation:

| Claim | Implementation reference |
|---|---|
| Commitment-only public export | `mvp_vault.rs` — `export_public_commitments()` |
| Deterministic local replay | `run_mvp_demo.sh` — two-run root-stability check |
| One-receipt selective disclosure | `mvp_vault.rs` — `disclose_receipt()` |
| Offline-first design | No network calls in `demo.rs` or `mvp_vault.rs` |

All blocked claims (payment, genesis admission, production ZK, production attestation, network deployment) are absent from the implementation.

## Conclusion

No cross-domain contamination detected.  
No private body text in any public output path.  
Claims boundary correctly enforced in both code and documentation.

The MVP baseline is approved for use as the TRL 5 hardening reference.
