# Data Handling Terms — QASH Pilot

These terms apply to all pilot evaluations of the QASH offline incident-log commitment demonstrator.

---

## What Data the Pilot Produces

| Artifact | Content | Classification |
|---|---|---|
| `public_commitments.bin` | SHA3-256 commitment roots only — no incident body | Public |
| `replay.json` | Replay root and record count — no incident body | Public |
| `disclosure.bin` | One decrypted receipt body (synthetic data) | Confidential |

The system is designed so that **no private incident body ever appears in any public artifact.** The `build_pilot_evidence_bundle.sh` script enforces this programmatically.

---

## What Data Must Not Be Used

- Real cyber-incidents, production logs, or live operational data.
- Personally identifiable information of any kind.
- Data subject to regulatory retention or disclosure obligations.

The pilot is designed for **synthetic data only.** The demo script generates its own synthetic incident text.

---

## Data Minimisation

The QASH architecture is explicitly designed for data minimisation:

- The commitment root is the only value that leaves the operator's workspace.
- The incident body is encrypted in `disclosure.bin` and never exported to public files.
- No transaction graph, operator identity, or timing correlation is published.

---

## Retention and Deletion

- Pilot evaluation artefacts (`artifacts/pilot-baseline-v0.2/`) are non-persistent outputs.
- They are not committed to version control (covered by `.gitignore`).
- Partners may delete the entire `.qash-mvp-demo/` workspace at any time with no residual data.

---

## No Production Data Commitment

This pilot creates no commitment to use QASH with real incident data. Any future production deployment would require a separate data processing agreement and review against applicable data protection law.

---

All technical claims are governed by `docs/mvp/claims_register.md`.
