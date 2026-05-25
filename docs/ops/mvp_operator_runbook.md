# Operator Runbook: QASH Offline Incident-Receipt Commit Demonstrator

**Scope:** Domain B MVP demonstrator only.
**Claim boundary:** See `docs/mvp/claims_register.md`. This system is not a payment instrument, not a networked consensus deployment, and not a production system. All operations are local and offline unless the operator explicitly transfers files to another machine.

---

## 1. Prerequisites

### Rust toolchain

The workspace pins the Rust toolchain via `rust-toolchain.toml` at the repository root. The pinned toolchain is applied automatically by `rustup` when any `cargo` command is run from within the repository. If `rustup` is not installed, install it from `https://rustup.rs` before proceeding.

Verify the active toolchain resolves to the pinned version:

```sh
rustup show active-toolchain
```

The output must match the channel specified in `rust-toolchain.toml`. Do not override this with `+nightly` or `+stable` flags; the pinned version is required for deterministic build output.

### Building the `qash-demo` binary

```sh
cargo build --release -p qash
```

The binary is placed at `target/release/qash`. All demo subcommands are accessed through this binary using the `demo` subcommand prefix:

```sh
./target/release/qash demo <subcommand> [options]
```

For convenience, examples in this runbook use `qash-demo` as a shorthand alias. Add the binary to `PATH` or create an alias if preferred:

```sh
alias qash-demo='./target/release/qash demo'
```

### Test suite

Run the full workspace test suite before operating the MVP:

```sh
cargo test --workspace --no-default-features
```

All tests must pass. A failing test before operation indicates an environment or build configuration problem and should be resolved before proceeding.

---

## 2. Workspace Initialisation

### Command

```sh
qash-demo init --dir <workspace>
```

### What it does

Creates a workspace directory at `<workspace>` if it does not already exist, and initialises the following layout:

```
<workspace>/
  vault/                  # Private encrypted incident bodies (never transferred)
  disclosures/            # Selective disclosure bundles written by `disclose`
  commitments.wal         # Append-only write-ahead log of public commitment records
  vault_salt.bin          # 32-byte random salt used for nonce derivation (never transferred)
  manifest.txt            # Human-readable summary: workspace version, init timestamp, record count
```

### File responsibilities

| File | Domain | Purpose |
|------|--------|---------|
| `vault/` | Private | Stores encrypted incident body for each receipt. Required for disclosure. |
| `disclosures/` | Private | Output directory for `disclose` command. Contents are operator-controlled. |
| `commitments.wal` | Public-safe | Append-only log of `TxMvpReceiptCommit` public commitment records. May be exported. |
| `vault_salt.bin` | Private | Random 32-byte value. Used in nonce derivation. Loss permanently breaks nonce derivation. |
| `manifest.txt` | Informational | Summary record. Not used in cryptographic operations. |

`init` is idempotent if the workspace already exists and was created by the same binary version. It will refuse to reinitialise a workspace with an incompatible version tag.

---

## 3. Issuing a Receipt

### Command

```sh
qash-demo issue-receipt --dir <workspace> --epoch <n> --body "<text>"
```

`<n>` is a `u64` epoch number. `<text>` is the private incident body — the literal text of the incident log entry. The body is never written to `commitments.wal` or any exported file.

### Output

On success, the command prints:

```
receipt_id:          <64-character hex string>
payload_commitment:  <64-character hex string>
```

`receipt_id` is derived from the epoch, nonce, and domain tag. It is the stable identifier used to reference this receipt in subsequent `disclose` and `replay` operations.

`payload_commitment` is a domain-tagged SHA3-256 commitment to the private body. It appears in the public commitment record and in the WAL. The commitment cannot be reversed to recover the body without the disclosure key material stored in `vault/`.

### What is stored

| Location | Content | Private? |
|----------|---------|---------|
| `vault/<receipt_id>` | Encrypted incident body plus nonce preimage | Yes — never leave the workspace |
| `commitments.wal` | Appended `TxMvpReceiptCommit` record: `version`, `epoch`, `nonce` (hashed), `payload_commitment`, `disclosure_key_commitment`, `domain_tag` | Public-safe — no raw body or nonce preimage |

The nonce stored in the WAL is the commitment-layer nonce, not the raw nonce preimage. The raw nonce preimage required for disclosure is stored only in `vault/`.

### Epoch and nonce uniqueness

The system enforces that no two receipts in the same workspace share the same `(epoch, nonce)` pair. If a collision is detected, the command returns `DuplicateEpochNonce` and no record is written. Use a different epoch value or allow the system to derive a new nonce automatically.

---

## 4. Syncing to a Peer

### Command

```sh
qash-demo sync --dir <workspace> --peer-dir <peer>
```

### What it does

Reads all records from `<workspace>/commitments.wal` and writes a flat binary export file to `<peer>/public_commitments.bin`. This file contains only the public commitment records — one `TxMvpReceiptCommit` record per committed receipt, each exactly 140 bytes in fixed-size encoding.

### What `public_commitments.bin` contains

Each record encodes:
- `version: u32`
- `epoch: u64`
- `nonce` field: 32 bytes (commitment-layer nonce hash, not the raw preimage)
- `payload_commitment: [u8; 32]`
- `disclosure_key_commitment: [u8; 32]`
- `domain_tag: [u8; 32]`

### What `public_commitments.bin` does not contain

- Raw incident body text
- Raw nonce preimage
- Any vault-derived key material
- Operator identity, device identifier, or site identifier

The peer workspace receives only public commitment evidence. It can replay and verify the commitment root but cannot reconstruct incident bodies or produce disclosures.

---

## 5. Importing from a Peer

### Command

```sh
qash-demo sync --dir <peer> --import <path>
```

`<path>` is the path to a `public_commitments.bin` file received from another workspace.

### What it does

Appends the imported public commitment records to `<peer>/commitments.wal`. The peer workspace can subsequently run `replay` to compute and verify the commitment root. The peer cannot produce disclosures for imported records because the corresponding vault entries are absent.

If `disclose` is attempted against an imported-only `receipt_id`, the command returns `ReceiptNotFound`. This is expected behaviour: the vault entry for that receipt does not exist in the peer workspace, and no disclosure is possible without it.

### Integrity check

The import step validates each incoming record against the fixed-size encoding and domain tag before appending. Records that fail validation are rejected and the WAL is not modified.

---

## 6. Replay and Root Verification

### Command

```sh
qash-demo replay --dir <workspace>
```

With machine-readable output:

```sh
qash-demo replay --dir <workspace> --report report.json
```

### What it does

Reads all records from `commitments.wal`, folds them in append order using the commitment hash cascade (SHA3-256 chain over each record's `payload_commitment`), and outputs the resulting `commitment_root`.

### Output (text mode)

```
records_replayed:   <n>
commitment_root:    <64-character hex string>
halt_flag:          false
```

`commitment_root` is deterministic: any two workspaces with identical WAL contents will produce identical roots regardless of the machine, OS, or architecture on which they run, subject to Domain A arithmetic rules.

### Output (`--report` mode)

The `--report` flag writes a JSON file with the following structure:

```json
{
  "records_replayed": <n>,
  "commitment_root": "<hex>",
  "halt_flag": false,
  "epoch_range": [<min>, <max>],
  "replay_status": "ok"
}
```

`replay_status` is `"ok"` on success. It is `"invalid_wal"` if any record fails validation during replay. See section 10 for error handling.

### Interpreting the commitment_root

The root is a folded hash over commitment-only fields. It is evidence that a specific sequence of commitment records existed at the time of replay. It does not prove the content of incident bodies; that requires selective disclosure (section 7). Two parties comparing roots can confirm they hold identical public commitment transcripts without exchanging any private data.

---

## 7. Selective Disclosure

### Command

```sh
qash-demo disclose --dir <workspace> --receipt-id <hex> --out disclosure.bin
```

`<hex>` is the `receipt_id` printed by `issue-receipt`.

### What the disclosure bundle contains

`disclosure.bin` is a self-contained binary bundle including:

- The `receipt_id`
- The `epoch` and `payload_commitment` values from the WAL record
- The raw incident body (decrypted from `vault/`)
- The raw nonce preimage
- A domain tag and version field

The bundle is sufficient for a recipient to independently verify that:
1. The body hashes to `payload_commitment` under the domain-tagged commitment scheme.
2. The `receipt_id` matches the disclosed record.

### What the bundle does not contain

- Records for any other receipt
- Any vault key material beyond what is needed to verify this single receipt
- The contents of any other committed epoch

### Scope

Disclosure is limited to one receipt per invocation. Multiple disclosures require multiple `disclose` calls with different `--receipt-id` values. Importing a peer's `public_commitments.bin` does not grant disclosure capability for those records.

---

## 8. Backup and Recovery

### Files that must be backed up

Back up the following files from each workspace:

| File/Directory | Reason |
|---------------|--------|
| `vault/` | Contains encrypted bodies and nonce preimages. Without this, no disclosure is possible. |
| `vault_salt.bin` | Required for nonce derivation. Loss is not recoverable. |
| `commitments.wal` | The authoritative record of all commitments. Replay and disclosure both depend on it. |

`manifest.txt` and `disclosures/` are not cryptographically critical but should be included in backups for operational completeness.

`public_commitments.bin` (in a peer workspace) is re-derivable from `commitments.wal` via `sync`. It does not need to be independently backed up.

### Loss of `vault_salt.bin`

If `vault_salt.bin` is lost, nonce derivation for future receipts is broken. Existing vault entries cannot be re-keyed without re-issuing receipts. The workspace should be treated as unrecoverable for disclosure purposes. Restore from a backup that includes `vault_salt.bin`.

### Recovery from a truncated WAL

If `commitments.wal` is truncated (for example, due to a storage failure mid-write), the `replay` command will detect the incomplete trailing record and return `InvalidWal`. The system will not silently accept a truncated log.

Do not attempt to manually repair a truncated WAL. The WAL format uses fixed-size framed records; truncation mid-frame leaves an irrecoverable partial record. The only correct recovery is to restore `commitments.wal` from a backup. Records written after the truncation point are permanently lost; they cannot be recovered without the original `issue-receipt` input.

If no backup exists and partial data loss is acceptable, the truncated WAL must be discarded and a new workspace initialised. There is no mechanism to reconstruct lost records from `public_commitments.bin` alone.

---

## 9. Privacy Invariants

The following invariants hold across all MVP operations and must be preserved when extending or deploying the demonstrator:

1. **Commitment root is non-reversible.** The `commitment_root` is a folded hash over `payload_commitment` values. It cannot be reversed to recover incident bodies without the vault and nonce preimages.

2. **`public_commitments.bin` contains no raw incident text.** Every field in the exported commitment record is either a fixed-size hash or a fixed-size integer. No field carries incident narrative, operator metadata, or nonce preimages.

3. **Only the local vault can produce disclosures.** A peer that receives `public_commitments.bin` via `sync --import` can replay the commitment root but cannot produce disclosure bundles. Disclosure capability is exclusive to the workspace that issued the receipt.

4. **No stable user identity in public records.** `TxMvpReceiptCommit` has no validator ID, public key, account field, device serial, or operator identifier in any public-facing field.

5. **Domain B material does not enter Domain A arithmetic.** Private bodies, nonce preimages, and vault keys are not inputs to any computation that produces commitment records or the commitment root.

---

## 10. Troubleshooting

### `InvalidWal`

**Cause:** The WAL file is truncated, contains a malformed record, or has been corrupted (for example, by a partial write or storage error).

**Action:** Do not attempt manual repair. Restore `commitments.wal` from a backup. If no backup exists and the workspace is on the receiving end of an import (peer workspace), delete `public_commitments.bin` and re-import from the originating workspace.

The system will not accept a WAL with invalid records and will not proceed with replay or disclosure while the WAL is in this state.

### `ReceiptNotFound`

**Cause:** `disclose` was called with a `receipt_id` that has no corresponding vault entry in this workspace. This occurs when:
- The receipt was issued on a different machine and only the public commitment was imported.
- The `receipt_id` hex string is incorrect.
- The vault entry was deleted or the vault directory was not included in a restore.

**Action:** Verify the `receipt_id` against the output of a previous `issue-receipt` call. If the receipt was issued on another machine, disclosure must be performed there. Vault entries are not transferable between workspaces.

### `DuplicateEpochNonce`

**Cause:** An `issue-receipt` call attempted to use an `(epoch, nonce)` pair that already exists in the WAL for this workspace.

**Action:** Use a different `--epoch` value. If the nonce is derived automatically, this indicates that the same epoch has been used more than once in a context that exhausts the nonce space. In practice for the MVP, using a unique epoch value per receipt is the simplest mitigation.

### Mismatched `commitment_root` between two workspaces

**Cause:** The WAL contents differ between the two workspaces. Possible causes: the sync export was performed before some receipts were issued, an import was truncated, or records were issued locally after sync.

**Action:** Re-run `sync --dir <source> --peer-dir <peer>` to export a fresh `public_commitments.bin`, then re-import on the peer. Replay both workspaces and compare roots.

---

## 11. Claim Boundary

The allowed and blocked claims for this demonstrator are defined in full in `docs/mvp/claims_register.md`. This runbook references that register as the authoritative source.

Stated explicitly for operational clarity:

- This is not a payment system. No monetary value, settlement finality, or payment obligation is created or recorded.
- This is not networked consensus. The `sync` step copies a flat binary file between local directories or machines. There is no peer-to-peer protocol, no validator network, and no consensus participation.
- This is not a production deployment. No security certification, certified attestation chain, or incident-response procedure exists. The system is a demonstrator suitable for pilot discussion, not operational use.

Any use of this system outside the scope of the allowed claims in `docs/mvp/claims_register.md` requires a separate specification, proof obligation, and admissibility review.
