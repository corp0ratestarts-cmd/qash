# Pure QASH RC Evidence Snapshot

**Status:** Template — to be populated at RC milestone  
**Milestone:** pure-qash-v0.1-rc1  
**Genesis status:** provisional  
**Deployment authoritative:** false

---

## Evidence capture command

```sh
cargo xtask capture-evidence
```

This command produces a JSON evidence bundle at `docs/release/evidence_YYYYMMDD.json`.
It must exit 0 and produce no user graph material in its output.

---

## Required evidence fields

| Field | Source | Status |
|-------|--------|--------|
| Commit SHA | `git rev-parse HEAD` | ☐ |
| GENESIS_CONSTANTS hash | `cargo xtask verify-genesis` | ☐ |
| PublicTranscript field audit | `cargo test -- public_transcript` | ☐ |
| Zero-persistence gate results | CI `zero-persistence-gates` job | ☐ |
| Economics conservation tests | CI `economics-tests` job | ☐ |
| MEV-null fee validation tests | CI `economics-tests` job | ☐ |
| Absence guard results | CI `absence-guard` job | ☐ |
| Proof compilation (all TARGET files compile) | CI `proof-compilation` job | ☐ |
| Cross-ISA state root identity | CI `cross-isa-determinism` job | ☐ |
| Cargo deny / supply chain | CI `supply-chain` job | ☐ |
| Fuzz smoke (consensus core) | ☐ |
| Benchmark summary (no TX content) | ☐ |

---

## Forbidden from evidence bundle

The following MUST NOT appear in any evidence artifact:

```
Raw transactions or transaction lists
Receipt plaintext
Sender / receiver / amount records
Peer IP addresses
Socket addresses or routing metadata
Transaction timing logs
Raw WAL records (beyond schema-level summary)
Payload-bearing error messages
```

If `cargo xtask capture-evidence` would produce any of the above, it must
exit non-zero and report which field is forbidden.

---

## RC tag criteria

Tag `pure-qash-v0.1-rc1` may be created only after:

- [ ] All required evidence fields above are populated
- [ ] CI is green on the tagged commit
- [ ] `genesis_status = "provisional"` confirmed (no genesis lock)
- [ ] `deployment_authoritative = false` confirmed
- [ ] No regulated profile present (absence guards green)
- [ ] No external certification claimed

Do NOT create `pure-qash-v1.0-reference` without a separate
`[pure-qash-genesis-candidate-acknowledged]` PR decision.
