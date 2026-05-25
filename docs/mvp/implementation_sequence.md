# MVP Implementation Sequence

**Scope:** Offline incident receipt commit demonstrator.  
**Primary spec:** `docs/mvp/incident_receipt_commit_demo.md`.

## Strategic choice

The project will pursue **offline critical-infrastructure incident-log attestation** as the first MVP use case.

This is preferred over offline energy settlement receipts, emergency-service credentials, high-risk payment proofing, or a general post-quantum audit rail because it is the lowest-cost, lowest-regulatory, and fastest demonstrator that still exercises QASH's core differentiator: deterministic replay of commitment-only evidence without publishing a transaction graph.

## Slice order

### MVP-0 — Planning and claim boundary

**Branch:** `mvp-receipt-commit-demo-plan`

Deliver:
- `docs/mvp/incident_receipt_commit_demo.md`
- this implementation sequence
- roadmap/project-status references

Exit criteria:
- MVP is explicitly Domain B-only;
- no payment, settlement, credential, or production-network claim is made;
- `TX-MVP-ReceiptCommit` shape and CLI names are frozen for implementation.

### MVP-1 — CLI skeleton

**Branch:** `mvp-demo-cli-skeleton`

Deliver:
- `qash demo init`
- `qash demo issue-receipt`
- `qash demo sync`
- `qash demo replay`
- `qash demo disclose`

Exit criteria:
- commands parse;
- help text exists;
- placeholder handlers are explicit;
- no Domain A behavior changes.

### MVP-2 — Receipt commit type

**Branch:** `mvp-receipt-commit-type`

Deliver:
- Domain B `TxMvpReceiptCommit` type;
- deterministic fixed-size serialization;
- domain-tag validation;
- epoch-bound local nonce validation;
- tests proving the type has no stable user identity fields.

Exit criteria:
- serialization KAT exists;
- duplicate local nonce is rejected;
- public export contains only commitments.

### MVP-3 — Local vault and commitment WAL

**Branch:** `mvp-demo-vault-wal`

Deliver:
- private local receipt vault;
- append-only commitment log;
- corruption/truncation/duplication detection;
- default export path that excludes private payloads.

Exit criteria:
- `issue-receipt` creates private receipt body and commitment record;
- replay detects corrupted WAL records;
- exported sync material has no private payload.

### MVP-4 — Sync and replay

**Branch:** `mvp-demo-sync-replay`

Deliver:
- local workspace-to-workspace sync;
- commitment-only import/export;
- canonical replay report;
- two-workspace convergence test.

Exit criteria:
- same commitment log yields same public root in two workspaces;
- reordering/duplication/missing records are detected or canonically handled;
- replay report is suitable for evidence capture.

### MVP-5 — Selective disclosure

**Branch:** `mvp-demo-disclosure`

Deliver:
- disclose one selected private receipt;
- include matching commitment proof material;
- exclude unrelated receipts;
- local disclosure audit record.

Exit criteria:
- disclosure test proves only one receipt body is exported;
- unrelated local vault entries remain private;
- disclosure-key commitment is documented as placeholder only.

### MVP-6 — Evidence bundle

**Branch:** `mvp-demo-evidence`

Deliver:
- scripted full demo run;
- transcript leak tests;
- replay report artifact;
- benchmark-lite timings;
- allowed/blocked claims register;
- TRL uplift note for Innovate UK / EU equivalent applications.

Exit criteria:
- clean checkout demo succeeds;
- evidence bundle captures command output and root continuity;
- claims remain bounded to demonstrator status.

## Implementation discipline

- MVP work lives in Domain B unless explicitly reviewed otherwise.
- MVP must not alter `GENESIS_CONSTANTS.toml`.
- MVP must not add a production Domain A transaction type.
- MVP must not introduce public graph publication.
- MVP must not claim production privacy, settlement, attestation, or ZK verification.
- Every implementation slice should be small, mergeable, and independently testable.
