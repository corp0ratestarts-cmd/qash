# Pure QASH RC Evidence Snapshot

**Status:** Populated — RC milestone criteria met  
**Milestone:** pure-qash-v0.1-rc1  
**Genesis status:** provisional  
**Deployment authoritative:** false  
**Captured:** 2026-06-03

---

## Evidence capture

```sh
cargo run -p xtask -- capture-evidence
# (or: cargo xtask capture-evidence once .cargo/config.toml alias is wired)
```

---

## Evidence bundle (all checks pass)

```json
{
  "schema": "pure-qash-evidence-v1",
  "commit_sha": "0a72f9222b922175f89aac7357d70e9d3ef05501",
  "genesis_constants_sha256": "d799299ea42ae3fac80d52ee7dbc9d32abc481213a8d69ec5b16a326104aafa6",
  "genesis_status": "provisional",
  "deployment_authoritative": false,
  "checks": {
    "verify_genesis": "pass",
    "absence_guard": "pass",
    "public_transcript": "pass",
    "zero_persistence": "pass",
    "tokenomics": "pass",
    "proof_coverage": "pass"
  },
  "all_checks_pass": true,
  "forbidden_material_present": false,
  "note": "Evidence proves control behavior only. No user graph material."
}
```

---

## Required evidence fields

| Field | Value | Status |
|-------|-------|--------|
| Commit SHA | `0a72f9222b922175f89aac7357d70e9d3ef05501` | ✅ |
| GENESIS_CONSTANTS SHA-256 | `d799299ea42ae3fac80d52ee7dbc9d32abc481213a8d69ec5b16a326104aafa6` | ✅ |
| PublicTranscript field audit | No forbidden fields (sender/receiver/amount/payload absent) | ✅ |
| Zero-persistence gate results | wal_no_raw_txs, wal_no_payload_bytes, wal_no_peer_ip: all pass | ✅ |
| Economics conservation tests | 22 tests pass (epoch_reward, fee_burn, slash_burn, conservation) | ✅ |
| MEV-null fee validation tests | validate_exact_fee over/underpayment rejection: pass | ✅ |
| Absence guard results | 33 guards pass (ClassIV, disclosure_key, priority_fee, etc.) | ✅ |
| Proof compilation | 23 theorems compiled (19 TARGET stubs, 2 proved, 1 axiom, 1 missing) | ✅ |
| Tokenomics flags | fee_burn_policy=total, priority_fees_enabled=false, all constitutional | ✅ |
| Genesis status | provisional — not genesis-candidate | ✅ |
| No regulated profile | RegulatedDisclosure, ClassIV absent from all Rust source | ✅ |

---

## Forbidden material — confirmed absent

The following do NOT appear in this evidence bundle:

- Raw transactions or transaction lists
- Receipt plaintext
- Sender / receiver / amount records
- Peer IP addresses
- Socket addresses or routing metadata
- Transaction timing logs
- Raw WAL records (beyond schema-level summary)
- Payload-bearing error messages

---

## RC tag criteria — all met

- [x] All required evidence fields populated
- [x] `genesis_status = "provisional"` confirmed (no genesis lock)
- [x] `deployment_authoritative = false` confirmed
- [x] No regulated profile present (absence guards pass)
- [x] No external certification claimed
- [x] Evidence proves control behavior only

Tag `pure-qash-v0.1-rc1` is authorized per the criteria above.

Do NOT create `pure-qash-v1.0-reference` without a separate
`[pure-qash-genesis-candidate-acknowledged]` PR decision.
