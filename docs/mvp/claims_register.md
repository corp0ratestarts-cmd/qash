# MVP Claims Register

**Transaction type:** `TX-MVP-ReceiptCommit`  
**Domain:** B (demonstrator only — not a production Domain A transaction)  
**Status:** MVP baseline

This register defines the precise boundary of claims that are and are not supported by the QASH MVP offline incident receipt commit demonstrator. It exists to prevent scope creep, funding misrepresentation, and accidental admission of the MVP as a production system.

---

## Allowed Claims

The following claims are accurate and may be made about the MVP demonstrator:

| Claim | Basis |
|-------|-------|
| Local offline incident receipt demonstrator | `qash-demo init` / `issue-receipt` stores a commitment-only WAL locally |
| Commitment-only public export | `sync` exports `TxMvpReceiptCommitPublicExport` records; raw nonce and private body are excluded |
| Deterministic local replay | `replay` folds public commitment records into a stable root that is identical across runs and machines |
| One-receipt selective disclosure | `disclose` exports a single receipt body keyed by receipt ID without revealing other receipts |
| No stable public user identity in the MVP transaction type | `TxMvpReceiptCommit` has no validator ID, public key, or account field |
| Offline-first design | The full `init` → `issue-receipt` → `sync` → `replay` → `disclose` flow requires no network connectivity |
| Domain B demonstrator with no Domain A admission | The MVP type is not wired into `advance_epoch`; it cannot influence consensus state |

---

## Blocked Claims

The following claims are **not** supported by the MVP and must not be made:

| Blocked claim | Reason |
|---------------|--------|
| Production payment instrument | No regulated payment rail, settlement finality, or liability framework exists |
| Regulated financial settlement | No e-money, payment institution, or clearing authorisation |
| Production custody of assets | No asset issuance, custody accounting, or redemption logic |
| Production hardware attestation | TPM/TEE attestation paths are stubs; no certified attestation chain exists |
| Production ZK verification | No ZK proof verifier is wired; `ZkProofBundle` and related types are Domain B scaffolding |
| Genesis-admitted transaction | `TX-MVP-ReceiptCommit` has not undergone genesis admission review; it is not in `GENESIS_CONSTANTS.toml` |
| Full privacy proof for arbitrary transaction classes | Selective disclosure applies only to `TX-MVP-ReceiptCommit`; no general privacy proof exists |
| Production critical-infrastructure deployment | No security certification, operational runbook, or incident-response procedure exists |
| Stable public identity or credential system | No identity binding, revocation, or credential issuance is present in the MVP |
| Multi-party or networked consensus | The MVP sync step copies a flat commitment file; no peer-to-peer protocol exists |

---

## Funding Framing Guidance

When describing the MVP in funding applications (Innovate UK, EU Horizon, SBRI, etc.), use language from the allowed claims column only. The recommended framing is:

> QASH demonstrates a local cyber-resilience substrate for deterministic replay and selective disclosure of offline incident-log commitments without publishing a transaction graph.

Avoid the following terms in funding documents unless accompanied by the blocked-claims caveat above:

- "payment", "settlement", "clearing", "custody"
- "cryptocurrency", "token", "coin", "digital asset"
- "production-ready", "deployment-ready", "certified"
- "identity provider", "credential issuer"

---

## Verification

Run the end-to-end demo and assertion suite:

```bash
bash scripts/run_mvp_demo.sh
```

Expected output confirms:

- public commitments do not contain private incident body text
- selected disclosure contains only the selected receipt
- replay root is deterministic across two runs
