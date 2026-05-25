# MVP Demonstrator: Offline Incident Receipt Commit

**Status:** MVP planning baseline  
**Scope:** Domain B demonstrator transaction and hosted CLI flow only.  
**Non-goal:** This is not a genesis transaction type, payment instrument, credential system, settlement rail, or production network.

## Decision

The first MVP demonstrator is **offline site incident-log attestation**.

This is the simplest and lowest-cost path because it:

- avoids payment, custody, and market-settlement claims;
- avoids handling live personal credentials;
- works with a fixed-size payload commitment instead of a public application payload;
- exercises QASH's strongest existing property: deterministic replay of commitment evidence;
- fits cyber-resilience, offline assurance, and auditability funding language.

The demonstrator shows that an offline or disconnected site can commit to an incident record locally, sync only public commitments later, replay the same evidence on another node, and disclose a selected receipt to an auditor without publishing a transaction graph.

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
- `payload_commitment` is a fixed-size commitment to the private incident-log payload.
- `disclosure_key_commitment` is a placeholder commitment to a future disclosure capability.
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

## Strategic Implementation Order

1. Planning/docs slice: this document plus roadmap/status updates.
2. CLI skeleton slice: add `qash demo ...` commands with placeholder handlers and help text.
3. Receipt commit type slice: add Domain B `TxMvpReceiptCommit` serialization, validation, and tests.
4. Local vault/WAL slice: store private payloads locally and append commitment-only records.
5. Replay/sync slice: replay commitment log and sync public artifacts between two local workspaces.
6. Disclosure slice: disclose exactly one selected receipt with proof material.
7. Evidence slice: add scripted demo run, transcript leak tests, replay report, and bounded claims register.

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
- audit-trail MVP suitable for pilot discussion.

## Blocked Claims

The MVP must not claim:

- production payment capability;
- regulated settlement or e-money support;
- production hardware attestation;
- production ZK verification;
- genesis-lock readiness;
- complete privacy proof for arbitrary transaction classes;
- production deployment readiness.
