# Pilot Evidence Manifest — QASH Pilot Baseline v0.2

This manifest describes every artifact produced by the QASH pilot baseline demonstrator.
A third party can reproduce the full set by running `bash scripts/run_mvp_demo.sh --clean`.

## Artifacts

| Artifact | Path | Classification | Purpose |
|---|---|---|---|
| Public commitments export | `.qash-mvp-demo/public_commitments.bin` | **Public** | Machine-readable record of public commitment roots; safe to share |
| Replay report | `.qash-mvp-demo/replay.json` | **Public** | JSON replay result with `profile_version: 1`; `private_payloads_seen: false` guaranteed |
| Selective disclosure bundle | `.qash-mvp-demo/disclosure.bin` | **Selective** | Contains one decrypted receipt body; share only with the intended recipient |
| Claims register | `docs/mvp/claims_register.md` | **Public** | Authoritative list of what is and is not claimed; governs all partner-facing statements |
| Post-merge audit | `docs/mvp/post_merge_audit.md` | **Public** | Evidence that no private payload appears in public outputs |

## Privacy Boundary Rules

1. `public_commitments.bin` — must not contain any incident body text, raw nonces, workspace salts, or filesystem paths.
2. `replay.json` — `"private_payloads_seen": false` must be present and true; any `true` value is a claim boundary violation.
3. `disclosure.bin` — contains private receipt body; never include in public bundles or funder packets.

The `build_pilot_evidence_bundle.sh` script enforces these rules before writing output.

## Reproducibility

```sh
# Full reproduction from scratch (deterministic):
bash scripts/run_mvp_demo.sh --clean

# Verify replay root is stable:
bash scripts/run_mvp_demo.sh       # second run, same workspace
diff .qash-mvp-demo/replay.json /tmp/expected_replay.json

# Package public artifacts:
bash scripts/build_pilot_evidence_bundle.sh
```

Expected `replay.json` fields after a clean run:

```json
{
  "profile": "TX-MVP-ReceiptCommit",
  "profile_version": 1,
  "records": 2,
  "commitment_root": "<64-char lowercase hex>",
  "public_transcript_only": true,
  "private_payloads_seen": false,
  "status": "ok"
}
```

## What is NOT in this bundle

- No production Domain A genesis admission
- No production ZK proofs
- No production hardware attestation
- No real incident data or personally identifiable information
- No wallet keys, custody, payment, or settlement logic

See `docs/mvp/claims_register.md` for the full boundary definition.
