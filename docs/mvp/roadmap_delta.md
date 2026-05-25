# MVP Roadmap Delta: Incident Receipt Commit

**Status:** Planning delta for the main roadmap.  
**Primary scope document:** `docs/mvp/incident_receipt_commit_demo.md`.

## Near-term roadmap insertion

Before expanding production PAL, payment semantics, energy settlement, credential exchange, or genesis-lock work, QASH should first deliver a narrow MVP demonstrator:

```text
Offline critical-infrastructure incident-log attestation
```

The concrete demonstrator transaction is:

```text
TX-MVP-ReceiptCommit
```

This transaction is **Domain B-only demonstrator material**. It is not a production Domain A transaction, not TX-2, not a genesis-admitted transaction type, and not a regulated payment or settlement primitive.

## Why this becomes the first MVP

This is the simplest and most cost-effective option because it:

- avoids payment/e-money/custody regulation;
- avoids energy market settlement complexity;
- avoids live emergency-service credential handling;
- needs only fixed-size commitment records, not a full public payload model;
- maps cleanly to cyber-resilience and critical-sector audit funding language;
- exercises existing QASH strengths: deterministic replay, commitment-only public transcripts, and selective disclosure.

## MVP track

| Slice | Branch | Deliverable | Merge gate |
|---|---|---|---|
| MVP-0 | `mvp-receipt-commit-demo-plan` | scope docs and implementation order | no code behavior changes |
| MVP-1 | `mvp-demo-cli-skeleton` | `qash demo` command skeleton | commands parse; placeholders explicit |
| MVP-2 | `mvp-receipt-commit-type` | Domain B `TxMvpReceiptCommit` | deterministic serialization and no stable identity fields |
| MVP-3 | `mvp-demo-vault-wal` | local private vault + commitment WAL | private payload excluded from exports |
| MVP-4 | `mvp-demo-sync-replay` | local sync + replay report | two workspaces converge to same root |
| MVP-5 | `mvp-demo-disclosure` | selective one-receipt disclosure | unrelated receipts remain private |
| MVP-6 | `mvp-demo-evidence` | scripted demo + funding evidence bundle | clean checkout demo and bounded claims register |

## Relationship to existing roadmap

This MVP track does not replace the v1.1/v1.2 protocol roadmap. It sits before production deployment claims as a fundable demonstrator path.

The strategic sequence becomes:

1. keep current pre-genesis evidence gates green;
2. land MVP-0 documentation;
3. implement the five-command local CLI;
4. implement Domain B receipt commitment, vault, sync, replay, and disclosure;
5. produce an evidence bundle for Innovate UK / EU equivalent applications;
6. only then expand toward production PAL networking, hardware attestation, Plonky3 verification, and genesis-lock decisions.

## Claims allowed after MVP completion

- Offline incident-log commitment demonstrator.
- Deterministic replay of commitment-only evidence.
- Graph-non-publishing public transcript for the demo flow.
- Selective disclosure prototype.
- Critical-sector audit-trail MVP suitable for pilot discussion.

## Claims blocked after MVP completion

- Production payment network.
- Regulated settlement or e-money support.
- Production hardware attestation.
- Production ZK verification.
- Genesis-lock readiness.
- Complete privacy proof for arbitrary transaction classes.
- Production critical-infrastructure deployment readiness.
