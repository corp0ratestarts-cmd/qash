# Privacy Boundary Checklist — QASH

Run this checklist before any external sharing of QASH artifacts.

---

## Pre-Share Checklist

### 1. `public_commitments.bin`

- [ ] File does not contain any incident body text (check: `strings public_commitments.bin | grep -i incident` returns empty)
- [ ] File does not contain any raw nonces in plaintext
- [ ] File does not contain any workspace paths or operator identifiers
- [ ] File is the output of `qash sync`, not `qash disclose`

### 2. `replay.json`

- [ ] `"private_payloads_seen": false` is present
- [ ] `"status": "ok"` is present
- [ ] `"profile_version": 1` is present
- [ ] `commitment_root` is exactly 64 lowercase hex characters
- [ ] File does not contain the word "body", "incident", or any operator name

### 3. Evidence bundle (`artifacts/pilot-baseline-v0.2/`)

- [ ] `build_pilot_evidence_bundle.sh` exited 0 without error
- [ ] `disclosure.bin` is NOT included in the bundle
- [ ] Bundle only contains: `public_commitments.bin`, `replay.json`, `claims_register.md`, `post_merge_audit.md`

### 4. Selective Disclosure (`disclosure.bin`)

- [ ] Recipient has been identified and authorised
- [ ] Transmission is over an encrypted channel
- [ ] Bundle is not shared with any party other than the intended recipient

---

## Automated Checks

The following CI jobs enforce these checks on every PR:

| Check | Workflow | What it verifies |
|---|---|---|
| `zero-persistence-boundary` | `ci.yml` | No private data in public WAL records |
| `mvp-demo` | `mvp-demo.yml` | Full pipeline + bundle script privacy checks |
| `QASH domain boundary tripwires` | `genesis-guard.yml` | No Domain B value in Domain A state |

---

## Escalation

If any check above fails, do not share any artifacts. Contact `corp0rate.starts@gmail.com` before proceeding.

All claims are governed by `docs/mvp/claims_register.md`.
