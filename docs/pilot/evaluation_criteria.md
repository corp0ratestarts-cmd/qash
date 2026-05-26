# Evaluation Criteria — QASH Pilot

These criteria define what a successful pilot evaluation looks like. They are intentionally narrow and verifiable.

---

## Pass/Fail Criteria (All Must Pass)

| # | Criterion | How to Verify |
|---|---|---|
| 1 | Demo runs to completion without error | `bash scripts/run_mvp_demo.sh --clean` exits 0 |
| 2 | Replay report confirms no private payloads seen | `jq '.private_payloads_seen' .qash-mvp-demo/replay.json` returns `false` |
| 3 | Replay is deterministic | Run the demo twice; both `commitment_root` values are identical |
| 4 | Evidence bundle builds successfully | `bash scripts/build_pilot_evidence_bundle.sh` exits 0 |
| 5 | No private body text in public artifacts | Privacy checks in bundle script pass without error |

---

## Capability Assessment (Qualitative)

Beyond pass/fail, evaluators should consider:

1. **Offline operation** — does the system run without network access? (`unshare -n bash scripts/run_mvp_demo.sh --clean`)
2. **Selective disclosure** — can one receipt be revealed without revealing others? (inspect `disclosure.bin` vs `public_commitments.bin`)
3. **Replay independence** — can a third party replay `public_commitments.bin` without access to the original workspace?
4. **Evidence format** — is `replay.json` legible and auditable by non-technical stakeholders?
5. **Claim boundary clarity** — does `docs/mvp/claims_register.md` make the scope unambiguous?

---

## Out-of-Scope Criteria

The following are explicitly **not** evaluation criteria for this pilot:

- Performance under load (this is a demonstrator, not a benchmark)
- Production-grade post-quantum signature verification
- Real-time incident ingestion
- Regulatory compliance certification

---

## Feedback Template

After completing the evaluation, please provide feedback on:

1. Was criterion 1–5 met? If not, what failed?
2. Which capability mattered most for your use case?
3. What would make the replay report more useful?
4. What is missing before a production evaluation would be appropriate?
5. Would you be willing to co-author a brief case study?

Send feedback to `corp0rate.starts@gmail.com`.
