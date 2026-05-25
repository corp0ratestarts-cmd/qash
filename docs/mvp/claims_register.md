# MVP Claims Register

**Scope:** Offline incident receipt commit demonstrator.  
**Primary implementation:** `qash demo ...` local CLI flow.  
**Artifact script:** `scripts/run_mvp_demo.sh`.

## Supported claims after MVP completion

| Claim | Evidence source | Limitation |
|---|---|---|
| QASH has a local offline incident-log receipt commitment demonstrator | `scripts/run_mvp_demo.sh`; CLI output log | Local Domain B demo only |
| Private incident receipt bodies are not included in public commitment export | script leak check against `public_commitments.bin`; vault tests | Does not prove all future transaction classes are private |
| One selected receipt can be disclosed without disclosing unrelated receipts | `disclosure.bin` checks in `scripts/run_mvp_demo.sh`; vault tests | Placeholder disclosure-key commitment only |
| Commitment records can be replayed into a deterministic local commitment root | `qash demo replay`; run log | MVP root is a demo commitment root, not genesis consensus state |
| The demonstrator is suitable for pilot discussion | scope docs and repeatable local run | Not production deployment readiness |

## Blocked claims

| Blocked claim | Reason |
|---|---|
| Production payment capability | MVP has no payment semantics and no settlement scope |
| Custody support | MVP stores local demo receipts only and does not custody value |
| Production network deployment | MVP uses local workspaces and file-based exports |
| Production hardware attestation | MVP does not verify real hardware attestations |
| Production ZK verification | MVP does not verify production proof systems |
| Genesis-lock readiness | MVP remains pre-genesis and Domain B-only |
| Complete privacy proof for arbitrary transaction classes | MVP proves only local transcript-boundary behavior for this demo flow |

## Recommended wording

Use:

> A local cyber-resilience demonstrator showing deterministic replay and selective disclosure of offline incident-log commitments without publishing a transaction graph.

Avoid:

> A production currency, payment system, settlement layer, or deployed critical-infrastructure network.

## TRL note

The MVP moves QASH from a protocol/proof integration scaffold toward a TRL 5/6 demonstrator candidate. External pilot validation is still required before claiming a relevant-environment TRL 6/7 posture.
