# MVP Demonstrator: Offline Incident Receipt Commit

**Status:** MVP planning baseline  
**Scope:** Domain B demonstrator transaction and hosted CLI flow only.  
**Non-goal:** This is not a genesis transaction type, payment instrument, credential system, regulated settlement rail, or production network.

## Decision

The MVP demonstrator is **offline critical-infrastructure incident-log attestation**.

This is the simplest and lowest-cost path among the candidate demonstrators because it:

- avoids payment, e-money, custody, and energy-settlement regulation;
- avoids handling live personal credentials;
- works with a fixed-size payload commitment instead of a full application payload;
- exercises QASH's strongest existing property: deterministic replay of commitment evidence;
- fits Innovate UK / EU deep-tech framing as cyber resilience, offline assurance, and critical-sector auditability.

The demonstrator proves that an offline or disconnected industrial site can commit to an incident record locally, sync only public commitments later, replay the same evidence on another node, and disclose a selected receipt to an auditor without publishing a transaction graph.

## MVP Transaction Shape

The demonstrator transaction is named:

```text
TX-MVP-ReceiptCommit
```

It is a Domain B demonstrator artifact. It must not be admitted as a production Domain A transaction type until a separate post-MVP specification, proof obligation, and genesis admission review exist.

### Fields

```text
struct TxMvpReceiptCommit {
    version: u32,
    epoch: u64,
    nonce: [u8; 32],
    payload_commitment: [u8; 32],
    disclosure_key_commitment: [u8; 32],
    domain_tag: [u8; 32],
}
```

### Field rules

- `version` is fixed for the MVP demo profile.
- `epoch` is the epoch in which the incident receipt is admitted.
- `nonce` is epoch-bound and must be unique within the local demo vault.
- `payload_commitment` is a fixed-size commitment to the incident-log payload. The payload itself is private Domain B material.
- `disclosure_key_commitment` is a placeholder commitment to a future disclosure capability. No production disclosure-key scheme is claimed by MVP.
- `domain_tag` binds the transaction to the incident-log demonstrator profile.

### Identity rule

`TX-MVP-ReceiptCommit` has **no stable user identity**.

The MVP must not include sender, receiver, operator, device serial number, employee ID, site ID, or account ID in any public transcript field. Such data may exist only in private Domain B demo payloads and must be represented publicly only by commitments.

### Public output

The public demonstrator output is commitment-only:

```text
PublicTranscript(epoch_t) = {
  state_root_t,
  receipt_root_t,
  efb_root_t,
  epoch_t,
  halt_flag_t
}
```

No raw incident payload, nonce preimage, local device identifier, or disclosure material may appear in the public transcript.

## CLI Scope

The MVP CLI commands are:

```bash
qash demo init
qash demo issue-receipt
qash demo sync
qash demo replay
qash demo disclose
```

### `qash demo init`

Initializes a local demo workspace.

Expected effects:

- creates a local demo data directory;
- initializes a private receipt vault;
- initializes an append-only commitment log;
- writes a demo profile manifest;
- records the current binary version and commit metadata when available.

### `qash demo issue-receipt`

Creates one private incident receipt and appends its commitment.

Expected effects:

- accepts or generates a private incident payload;
- computes `payload_commitment`;
- derives or generates an epoch-bound nonce;
- records a private receipt body in the local vault;
- appends a `TX-MVP-ReceiptCommit` commitment record;
- prints only commitment metadata by default.

### `qash demo sync`

Synchronizes public commitment artifacts between demo nodes or local workspaces.

Expected effects:

- exports/imports commitment-only records;
- never exports private payload bodies by default;
- preserves deterministic ordering;
- records sync metadata as Domain B evidence only.

### `qash demo replay`

Replays the local commitment log through the canonical replay path.

Expected effects:

- recomputes the expected public roots;
- verifies root continuity;
- detects missing, duplicated, corrupted, or reordered records;
- emits a replay report suitable for the MVP evidence bundle.

### `qash demo disclose`

Selectively discloses a single private incident receipt to an auditor.

Expected effects:

- resolves a receipt by local receipt ID or commitment;
- exports the private receipt body and matching commitment proof;
- does not disclose unrelated receipts;
- records the disclosure event as local Domain B audit metadata.

## Strategic Implementation Order

1. **Planning/docs slice**: this document plus roadmap/status updates.
2. **CLI skeleton slice**: add `qash demo ...` commands with placeholder handlers and help text.
3. **Receipt commit type slice**: add Domain B `TxMvpReceiptCommit` serialization, validation, and tests.
4. **Local vault/WAL slice**: store private payloads locally and append commitment-only records.
5. **Replay/sync slice**: replay commitment log and sync public artifacts between two local workspaces.
6. **Disclosure slice**: disclose exactly one selected receipt with proof material.
7. **Evidence slice**: add scripted demo run, transcript leak tests, replay report, and funding-ready evidence bundle.

## Definition of Done

The MVP demonstrator is complete when:

- a clean checkout can run the five CLI commands;
- two local workspaces replay the same commitment log to the same root;
- restart/replay detects corruption or duplication;
- public transcript tests prove raw incident payload fields are absent;
- one receipt can be selectively disclosed without disclosing others;
- CI captures deterministic replay, transcript-boundary, and evidence-bundle checks;
- documentation states allowed claims and blocked claims for funding applications.

## Allowed Claims

After implementation, the project may claim:

- offline incident-log commitment demonstrator;
- deterministic replay of commitment evidence;
- graph-non-publishing public transcript for the demo flow;
- selective disclosure prototype;
- critical-sector audit-trail MVP suitable for pilot discussion.

## Blocked Claims

The MVP must not claim:

- production payment capability;
- regulated settlement or e-money support;
- production hardware attestation;
- production ZK verification;
- genesis-lock readiness;
- complete privacy proof for arbitrary transaction classes;
- production critical-infrastructure deployment readiness.
