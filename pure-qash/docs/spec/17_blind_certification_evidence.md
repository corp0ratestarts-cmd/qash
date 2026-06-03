# Pure QASH Blind Certification Evidence Boundary

**Status:** Normative  
**Scope:** What evidence Pure QASH may produce for audit, certification, and release purposes.

Core rule:
```
Certification evidence may prove control behavior.
Certification evidence may not preserve user behavior.
```

---

## §17.1 — Allowed evidence

The following evidence types are permitted in Pure QASH audit, release, and CI artifacts:

```
Build hashes (reproducible build SHA-256 per crate and binary)
Proof hashes (Coq proof artifact hashes, captured in proofs/artifact-index/)
CI results (pass/fail per job, no raw transaction data)
Replay vectors (state roots only; no TX payload)
Cross-ISA state-root evidence (x86_64 / aarch64 / riscv64gc identity)
Zero-persistence gate results (test pass/fail; no payload in output)
PublicTranscript field audit (confirms root-only boundary)
WAL redaction test results (confirms forbidden fields absent)
Dependency risk register (cargo-deny / OSV output)
Formal proof coverage (theorem status table from proofs/STATUS.md)
KATs (known-answer tests; fixed inputs/outputs only)
Benchmark summaries (throughput, latency — no TX content)
Absence guard results (pass/fail per forbidden-term check)
```

---

## §17.2 — Forbidden evidence in Pure QASH

The following MUST NOT appear in any audit report, CI artifact, evidence bundle,
or release document:

```
Raw transactions or transaction lists
Receipt plaintext
Sender / receiver / amount records
Peer IP addresses
Socket addresses or routing metadata
Transaction timing logs
Raw WAL records (beyond schema-level summary)
Payload-bearing error messages
Disclosure logs
Lawful-basis disclosures
Graph fragments or edge records
```

---

## §17.3 — Evidence bundle composition

A compliant Pure QASH evidence bundle contains only allowed evidence.
See `docs/release/pure_qash_rc_evidence_snapshot.md` for the RC evidence template.

The `cargo xtask capture-evidence` command produces a conforming bundle.
If any forbidden evidence type appears in the bundle output, the xtask command exits non-zero.

---

## §17.4 — Audit without user data

Pure QASH is designed to be auditable without retaining user graph material.
An external auditor can verify:
- Cryptographic correctness (build hashes, proof compilation, KATs)
- Protocol correctness (replay vectors, cross-ISA determinism)
- Privacy boundary correctness (zero-persistence gates, WAL redaction, PublicTranscript audit)
- Tokenomics correctness (economics unit tests, conservation invariant tests)
- Absence of forbidden concepts (absence guard CI results)

None of these verification steps requires access to real transaction data.
