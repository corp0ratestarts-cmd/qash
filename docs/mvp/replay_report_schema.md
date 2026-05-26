# Replay Report JSON Schema

**Version:** 1  
**Profile:** TX-MVP-ReceiptCommit (Domain B demonstrator)  
**Governing claims:** `docs/mvp/claims_register.md`

## Required fields

| Field | Type | Value |
|---|---|---|
| `profile` | string | `"TX-MVP-ReceiptCommit"` |
| `profile_version` | integer | `1` |
| `records` | integer | number of public export records replayed |
| `commitment_root` | string | 64-character lowercase hex SHA3-256 root |
| `public_transcript_only` | boolean | `true` |
| `private_payloads_seen` | boolean | `false` |
| `status` | string | `"ok"` |

Fields are emitted in the order above.

## Example

```json
{
  "profile": "TX-MVP-ReceiptCommit",
  "profile_version": 1,
  "records": 2,
  "commitment_root": "a3f1...c4d2",
  "public_transcript_only": true,
  "private_payloads_seen": false,
  "status": "ok"
}
```

## Constraints

- `commitment_root` must be exactly 64 lowercase hex characters.
- `private_payloads_seen` must be `false`; any `true` value is a claim boundary violation.
- No private incident bodies, raw nonces, workspace salts, disclosure bodies, or filesystem paths may appear anywhere in the report.
- The report is produced by `qash-demo replay --report <path>` and verified by `replay_report_json_schema_has_required_fields` in `crates/pal/src/mvp_vault.rs`.

## Claim boundary

This report schema is part of the Domain B demonstrator only. It does not constitute a production audit log, payment receipt, settlement record, or genesis-admitted transaction proof. See `docs/mvp/claims_register.md`.
