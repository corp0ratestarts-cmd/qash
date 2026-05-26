# QASH Pilot Package

**Version:** Pilot Baseline v0.2  
**Last updated:** 2026-05-26  
**Scope:** Offline cyber-incident evidence integrity demonstrator

---

## What QASH Demonstrates

QASH is a cyber-resilience substrate for offline incident-log commitments. The pilot demonstrates:

1. **Offline incident receipt issuance** — an operator issues cryptographic receipts for internal cyber-incidents without network access or external dependencies.
2. **Public commitment export** — the commitment root is exported without revealing any incident body text, log content, or operator identity.
3. **Deterministic replay** — a third party can replay the public commitment file and verify the root matches, using only the published replay profile.
4. **Selective disclosure** — the operator can reveal one receipt body to a specific recipient without disclosing others.
5. **Privacy boundary confirmation** — automated checks confirm that no private payload appears in any public artifact.

---

## Partner Responsibilities

| What the partner provides | Details |
|---|---|
| One or more notional incident scenarios | Synthetic data only — no real incidents required |
| Access to a Linux or macOS workstation | Rust toolchain installed via `rustup` |
| 30–60 minutes for installation and demo run | See `scripts/run_mvp_demo.sh` |
| Feedback on the evidence format | What would make it useful for your workflow? |

---

## What Data Must Not Be Shared

- `disclosure.bin` — contains a decrypted receipt body; treat as confidential
- Any incident body text used during the demo run
- Workspace salts and nonces generated during the run (not exported by design)

The `public_commitments.bin` and `replay.json` files are safe to share externally.

---

## What Outputs Are Produced

| Output | Format | Share? |
|---|---|---|
| `public_commitments.bin` | Binary commitment export | Yes |
| `replay.json` | JSON replay report | Yes |
| `disclosure.bin` | Binary selective disclosure | Designated recipient only |
| Evidence bundle (`artifacts/pilot-baseline-v0.2/`) | Directory of public artifacts | Yes |

---

## Success Criteria

A successful pilot run satisfies all five criteria:

1. `bash scripts/run_mvp_demo.sh --clean` exits 0.
2. `replay.json` contains `"status": "ok"` and `"private_payloads_seen": false`.
3. `bash scripts/build_pilot_evidence_bundle.sh` exits 0 and produces `artifacts/pilot-baseline-v0.2/`.
4. The `commitment_root` in `replay.json` is a 64-character lowercase hex string.
5. Running the demo a second time produces an identical `commitment_root`.

---

## Out of Scope

- Production deployment or live incident ingestion
- Payment, settlement, custody, or financial transactions of any kind
- Production post-quantum signature verification (signatures are carried opaquely in Domain A)
- Production hardware attestation (TPM/TEE integration is a Phase 2 item)
- Production identity, KYC, or regulatory compliance
- Genesis-admitted validator participation

All claims are governed by `docs/mvp/claims_register.md`.

---

## Installation

```sh
git clone https://github.com/corp0ratestarts-cmd/qash
cd qash
rustup toolchain install   # installs pinned toolchain from rust-toolchain.toml
cargo build --release
bash scripts/run_mvp_demo.sh --clean
```

Expected final output:

```
QASH MVP replay report
workspace: .qash-mvp-demo
records: 2
commitment_root: <hex>
status: deterministic local replay completed
```

---

## Contact

For pilot enquiries, contact `<pilot-contact-email>`.  
All partner-facing claims are governed by `docs/mvp/claims_register.md`.
