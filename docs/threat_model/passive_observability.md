# Threat Model: Passive Observability and Public Transcript Leakage

**Scope:** QASH MVP offline incident-receipt commit demonstrator (Domain B).
**Claim boundary:** See `docs/mvp/claims_register.md`.

---

## 1. Scope

This threat model covers information that can be inferred from the public transcript produced by the QASH MVP demonstrator, and from file-level observations of the sync workflow.

### What is in the public transcript

The public transcript consists of the records written to `public_commitments.bin` during a `sync` operation. Each record is a fixed-size `TxMvpReceiptCommit` encoding of exactly 140 bytes, containing:

- `version: u32` (4 bytes)
- `epoch: u64` (8 bytes)
- `nonce`: 32-byte commitment-layer nonce hash (not the raw preimage)
- `payload_commitment: [u8; 32]`
- `disclosure_key_commitment: [u8; 32]`
- `domain_tag: [u8; 32]`

All fields are fixed-width. The file size grows in exact multiples of 140 bytes as records are added.

### What is not in the public transcript

- Incident body text (the private payload passed to `issue-receipt --body`)
- Raw nonce preimage (stored only in `vault/`)
- Vault key material or vault salt
- Operator identity, device identifier, site identifier, or account field
- Absolute or relative timestamps of when individual receipts were issued
- Network addresses, hostnames, or filesystem paths from the issuing workspace

---

## 2. Assets

The following assets are relevant to this threat model:

| Asset | Location | Sensitivity |
|-------|----------|-------------|
| Private incident bodies | `vault/<receipt_id>` (encrypted) | High — operational incident narrative |
| Raw nonce preimages | `vault/<receipt_id>` | High — required to verify disclosure |
| `vault_salt.bin` | Workspace root | High — loss breaks nonce derivation |
| Epoch and nonce pairs | `vault/` only (hashed form in WAL) | Medium — reveals timing and volume |
| Operational timing patterns | Inferred from sync observations | Medium — reveals incident cadence |
| Identity of the vault operator | Not present in any public field | Medium — may be inferred by correlation |

---

## 3. Threat Actors

### Passive observer

A party who obtains a copy of `public_commitments.bin` through any means: file transfer, storage access, or observation of an unencrypted file system. The passive observer cannot mount active attacks and cannot modify the transcript or intercept future syncs.

### Active observer

A party who can observe multiple sync events over time — for example, by monitoring a shared directory, a file transfer channel, or network traffic carrying `public_commitments.bin` files. The active observer can correlate the growth of the file across multiple observations.

In the MVP, both actor types are assumed to have read-only access to `public_commitments.bin`. Neither actor has access to `vault/`, `vault_salt.bin`, or `commitments.wal` unless those files are also transferred, which the sync workflow explicitly does not do.

---

## 4. Threat T-PO-1: Record Count Leakage

**Description:** The number of 140-byte records in `public_commitments.bin` is directly observable from the file size. An observer can compute `record_count = file_size / 140` exactly. This reveals how many incident receipts have been committed in the workspace, regardless of the content of those receipts.

**Impact:** An observer learns the volume of committed incidents without learning what any incident contains. For high-sensitivity operational contexts, the incident count itself may be sensitive (for example, it reveals whether a reporting period was quiet or active).

**Mitigations:**

- *Batch commits within a single epoch.* If multiple incidents occur close together, issuing all receipts before performing a sync reduces the information gain from any single observation. The observer sees a batch, not individual events.
- *Pad with synthetic no-op records.* Inserting fixed-pattern records with domain-tagged no-op payloads would obscure the true record count. This is **not yet implemented** in the MVP. It is noted here as a future mitigation for contexts where record count is sensitive.

**Residual risk:** Record count is unavoidably visible to any party who receives `public_commitments.bin`. No mitigation eliminates this without changing the sync protocol.

---

## 5. Threat T-PO-2: Timing Correlation

**Description:** If sync events are observable — for example, if `public_commitments.bin` is transferred over a monitored network channel or written to a monitored file system — the timing of new records appearing in the file may correlate with the real-world occurrence of incidents.

An active observer who sees the file grow from N records to N+3 records at time T can infer that three incidents were committed at or before T. If the operational context is known (for example, the observer knows this workspace corresponds to a specific site), timing correlation narrows the inference.

**Impact:** Temporal correlation between sync events and incident occurrence. The body content remains protected, but the pattern of activity is partially visible.

**Mitigations:**

- *Delayed or batched sync.* Perform sync on a fixed schedule (for example, daily) rather than immediately after each receipt. This decouples the timing of individual receipts from the timing of public observations.
- *Offline-only transfer.* Transfer `public_commitments.bin` via physical media rather than a network channel. This removes the possibility of network-layer timing observation.

**Residual risk:** Any observable sync event reveals a lower bound on the number of new incidents since the last sync. Delayed sync reduces the precision of this inference but does not eliminate it.

---

## 6. Threat T-PO-3: Commitment Linkability

**Description:** The `disclosure_key_commitment` field appears in every public record. If the same `disclosure_key_commitment` value appears across multiple records, an observer can link those records as having originated from the same operator or vault, without knowing what any record contains.

**Impact:** A static `disclosure_key_commitment` value acts as a pseudonymous identifier across records. An observer can group records by this value and infer that they were produced by the same issuing party.

**Mitigations:**

- *Use fresh disclosure commitments per record.* Deriving a distinct `disclosure_key_commitment` for each receipt breaks the linkability chain. The current MVP implementation **may allow reuse** of a disclosure key commitment across receipts. This is a known limitation documented here.

**Known limitation:** The MVP does not enforce or automatically rotate `disclosure_key_commitment` values between receipts. Operators who require unlinkability between records must use the MVP with awareness of this limitation and plan for a future implementation that enforces per-record commitment freshness.

---

## 7. Threat T-PO-4: Commitment-Body Correlation via External Oracle

**Description:** An adversary who independently knows the content of an incident body — for example, through an external breach notification, an independent report, or other disclosure — can compute the expected `payload_commitment` for that body and search the public transcript for a matching entry. A match confirms that the body was committed to this workspace.

**Impact:** Confirmation of presence. The adversary does not learn the body from the transcript; they already possess it. The threat is confirmation that a specific incident was committed, which may be operationally significant.

**Mitigations:**

- *Domain-tagged commitment.* `payload_commitment` is a domain-tagged SHA3-256 commitment. For an adversary to mount this attack, they must know the exact body text, the domain tag, and the nonce preimage. Without the nonce preimage (stored only in `vault/`), the computation cannot be completed. The commitment scheme makes this attack computationally equivalent to a SHA3-256 preimage attack on the nonce-salted input, which is infeasible with current technology.

**Residual risk:** If the nonce preimage is leaked (for example, through a vault compromise or a disclosure bundle that is forwarded unintentionally), this protection degrades to the strength of the domain-tagged hash alone.

---

## 8. Threat T-PO-5: Import-Path Metadata Leakage

**Description:** When `public_commitments.bin` is transferred to a peer workspace and imported, the file is present on the peer's file system. The file size (and therefore record count) is visible to anyone with read access to that file system. Additionally, the file name, modification timestamp, and inode metadata may reveal when the import occurred.

**Impact:** A party with access to the peer's file system learns the record count and the approximate import time, even if the transfer was otherwise controlled.

**Mitigations:**

- *Transfer via encrypted channel.* Wrapping the file in an encrypted container (for example, GPG-encrypted archive) before transfer conceals the file size from a network observer. After decryption at the destination, the file size is again visible on the local file system.
- *Strip or normalise filesystem metadata.* Setting a fixed modification timestamp and stripping extended attributes before storing the imported file reduces the metadata surface. This is not automated by the MVP import step.

**Residual risk:** File size on the local file system of the receiving workspace is not protected by any mechanism in the current MVP.

---

## 9. Out of Scope

The following threats are explicitly out of scope for this document:

- **Active manipulation of the transcript.** An adversary who can modify `commitments.wal` or `public_commitments.bin` to insert, delete, or reorder records. This is covered by WAL integrity checks (fixed-size framing and domain tag validation) and is addressed in the replay integrity section of the operator runbook.
- **Domain A nondeterminism.** The threat that two conforming operators compute different commitment roots from the same WAL. This is covered by `docs/threat_model/nondeterminism.md`.
- **ZK proof integration.** Threats arising from a zero-knowledge proof layer over the public transcript. ZK proof integration is not in MVP scope. If added in a future version, a separate threat model extension is required.
- **Active network attackers.** Man-in-the-middle modification of `public_commitments.bin` in transit. The MVP does not include a transport security layer; operators requiring integrity in transit must apply standard controls (TLS, signed archives) independently.

---

## 10. Residual Risks

The following risks remain after applying the mitigations described above:

| Risk | Status |
|------|--------|
| Record count visible from file size | Unavoidable in current protocol; no padding scheme implemented |
| Timing correlation via observed sync events | Mitigated by delayed/batched sync; not eliminated |
| Commitment linkability via reused `disclosure_key_commitment` | Known MVP limitation; per-record rotation not yet implemented |
| Import file size visible on peer file system | Not mitigated by MVP tooling; operator must apply external controls |

These residual risks are acceptable for the MVP demonstrator scope. They must be reassessed before any deployment that handles operationally sensitive incident data beyond the demonstrator context.

---

## 11. Claim Boundary

The allowed and blocked claims for the QASH MVP are defined in full in `docs/mvp/claims_register.md`. This threat model is scoped to the MVP demonstrator and does not cover production deployment, networked consensus, payment systems, or ZK-proof-integrated transcript verification. Any extension of the MVP that materially changes the public transcript structure, the sync protocol, or the disclosure scheme requires a corresponding update to this threat model.
