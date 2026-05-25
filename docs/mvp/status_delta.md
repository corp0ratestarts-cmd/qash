# MVP Status Delta: Offline Incident Receipt Commit

**Status:** Planning delta for `PROJECT_STATUS.md`.  
**Primary scope document:** `docs/mvp/incident_receipt_commit_demo.md`.

## Current MVP decision

QASH's first funding-oriented MVP should be an **offline incident-log attestation** demonstrator.

The demonstrator centers on a Domain B artifact named:

```text
TX-MVP-ReceiptCommit
```

It is not a production Domain A transaction type and must not be represented as a payment, settlement, credential, or genesis-admitted transaction.

## Why this is the most strategic first step

The current project status already identifies operational immaturity as the highest deployment blocker. The fastest way to reduce that risk without creating unnecessary regulatory surface is to build a local hosted demonstrator that exercises:

- private receipt creation;
- commitment-only public sync;
- deterministic replay;
- local crash/replay evidence;
- one-receipt selective disclosure;
- transcript-boundary tests.

This advances the project toward a credible TRL 5/6 cyber-resilience demonstrator while preserving the pre-genesis claim boundary.

## Status table addition

| Dimension | Status | Notes |
|-----------|--------|-------|
| MVP demonstrator | **Planned** | Offline incident-log receipt commitment chosen as the first fundable demonstrator. `TX-MVP-ReceiptCommit` remains Domain B-only and avoids payment, market settlement, and credential-handling claims. |

## Strategic execution order addition

Before production PAL networking, hardware attestation, Plonky3 verification, payment semantics, or genesis lock:

1. land MVP-0 docs;
2. add five-command CLI skeleton;
3. implement Domain B receipt-commit type;
4. add local private vault and commitment WAL;
5. add local sync and deterministic replay report;
6. add one-receipt selective disclosure;
7. capture a funding-ready evidence bundle.

## Allowed near-term claim

QASH is preparing a funding-oriented MVP showing deterministic replay of offline incident-log commitments with graph-non-publishing public transcripts and selective disclosure.

## Blocked near-term claims

QASH is not yet claiming production deployment, regulated payment/settlement capability, production hardware attestation, production ZK verification, or genesis-lock readiness.
