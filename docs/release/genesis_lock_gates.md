# Genesis Lock Gates

This table is the release single source of truth for the non-proof blockers listed in `proofs/STATUS.md` under **Genesis Lock Requirement**.

| Blocker | Owner | Objective pass criterion | Command / artifact proving closure |
|---|---|---|---|
| Traceability artifact reconciliation | Protocol QA + Audit | `docs/traceability.md` rows that are part of P0 genesis readiness have no unresolved blocker markers for required lock gates, and each row has linked code and test/vector evidence. | `./scripts/check_document_hygiene.sh` and committed `docs/traceability.md` diff showing reconciled P0 rows. |
| Normative PDF finalization | Spec Governance | `spec/pdf/QASH_Spec_v1.0.pdf` is committed and `spec/pdf/README.md` authority and lock procedure is satisfied, with provisional references reconciled. | Presence of `spec/pdf/QASH_Spec_v1.0.pdf`; `./scripts/verify_genesis_hash.sh`; release packet section in `docs/release/rc_checklist_pack.md`. |
| Cross-ISA replay evidence review | Runtime Verification | Cross-ISA replay verification passes for `x86_64`, `aarch64`, and `riscv64gc` and matching roots are recorded in release artifacts. | `./scripts/verify_cross_isa_identity.sh x86_64-unknown-linux-gnu`; `./scripts/verify_cross_isa_identity.sh aarch64-unknown-linux-gnu`; `./scripts/verify_cross_isa_identity.sh riscv64gc-unknown-linux-gnu`; CI artifact bundle from platform determinism workflow. |
| PAL/network readiness decision | PAL + SRE | Explicit production go/no-go decision is recorded with networking/attestation readiness and residual risk acceptance in release records. | Signed readiness decision in `docs/release/rc_checklist_pack.md` and `docs/release/pre_genesis_evidence_snapshot.md` (decision section). |
