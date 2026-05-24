# QASH Zero-Persistence Execution Pipeline
## `docs/spec/14_zero_persistence_pipeline.md` — Domain B Production Profile Draft

> **Status:** Derived engineering specification for Domain B production hardening.
> This document is a Domain B policy and implementation-boundary specification.
> It does not authorize Domain A behavior changes, consensus-byte changes, or
> public transcript expansion without separate traceability and proof review.

---

## §14.1 — Purpose

This specification defines **Zero-Persistence Graph Non-Publication** for QASH
Domain B.

The goal is not merely to encrypt or hide transaction-graph material. The goal
is to ensure that raw graph material is never promoted into a durable,
serializable, replayable, or public artifact. Raw envelopes may exist only inside
an owned ephemeral admission lifecycle. The only admissible outputs of that
lifecycle are fixed-width scalar commitments, `CapToken<ValidatedEffect>` values,
blind audit events, and public transcript roots.

This is a production-mode constraint. The current hosted PAL replay scaffold may
retain raw fixtures under explicit testing features, but that mode is not a
zero-persistence deployment profile.

---

## §14.2 — Definitions

**Raw graph material** means any value that can reveal transaction topology,
receipt body contents, graph edges, raw envelopes, peer/network metadata,
transaction lists, routing topology, or payload-bearing error context.

**Ephemeral envelope** means the owned, non-serializable, non-cloneable, raw
admission buffer that temporarily contains encrypted or decrypted envelope bytes
inside Domain B.

**Validated effect** means the fixed-width, schema-validated result of admission.
It contains only scalar commitments and bounded public metadata required by
Domain A.

**Zero-persistence production mode** means a PAL build profile in which raw graph
material is not written to WAL, logs, metrics, traces, crash dumps, audit events,
or public transcripts.

**Sovereign Hardened profile** means the high-assurance deployment tier that adds
attested DPU/SmartNIC admission, confidential host memory, IOMMU lockdown, and
hardware-backed storage/erasure evidence to the zero-persistence software profile.

---

## §14.3 — Normative Invariants

### ZP-1: No durable raw graph material

Domain B MUST NOT persist, serialize, log, trace, metric-label, crash-dump, audit,
or publicly emit raw graph material.

Forbidden durable outputs include:

- raw envelopes or raw transaction bytes,
- graph edges or adjacency lists,
- receipt bodies or unencrypted receipt leaves,
- peer IP addresses, socket addresses, or routable peer identifiers,
- payload-bearing error messages,
- debug strings derived from raw envelope bytes,
- proof-byte contents unless separately admitted by a proof-backend spec.

### ZP-2: No owned payload copies in the admission path

The production admission path MUST NOT allocate or create owned copies of raw
envelope contents. Parsing must operate over borrowed views or fixed-size local
arrays. The admission path MUST NOT use `Vec<u8>`, `String`, `.to_vec()`,
`.clone()`, `Serialize`, `Debug`, `Display`, or equivalent trait paths for raw
envelope types.

### ZP-3: Commitment-only Domain A crossing

The only values that may cross from Domain B into Domain A are:

- `CapToken<ValidatedEffect>`,
- scalar effect roots,
- receipt roots,
- EFB roots,
- bounded epoch/profile metadata already admitted by the relevant protocol spec.

Domain A MUST NOT receive raw envelopes, cover traffic, traffic-shaping metadata,
network timing, peer metadata, proof-generation state, or indexer state.

### ZP-4: Blind audit only

Audit events MAY record boundary events such as admission success/failure class,
key zeroization, attestation status, halt trigger class, and shred commitment
publication. Audit events MUST NOT contain raw graph material or peer metadata.

### ZP-5: Feature bifurcation

The repository MUST preserve a strict distinction between replay scaffolding and
production zero-persistence:

- `replay-scaffold`: test/golden-vector mode; may use raw fixture WALs for
  evidence generation only.
- `zero-persistence`: production admission mode; WAL schema lacks raw payload
  fields and cannot compile code paths that persist raw graph material.
- `sovereign-hardened`: zero-persistence plus hardware admission and storage
  controls.

---

## §14.4 — Admission Lifecycle

The production lifecycle is:

```text
network ingress
  -> ephemeral admission slot
  -> in-place parser view
  -> schema validation
  -> scalar commitment extraction
  -> CapToken<ValidatedEffect>
  -> Domain A transition
  -> PublicTranscript roots
  -> slot zeroization and reuse/destruction
```

No stage in this pipeline may publish or persist raw graph material. Processing
and destruction are coupled by ownership: the raw envelope is consumed by value
by the admission airlock, and its storage is zeroized before the slot is returned
to a ring, allocator, or caller.

Reference Rust shape:

```rust
use core::marker::PhantomData;
use zeroize::Zeroizing;

pub struct EphemeralEnvelope {
    data: Zeroizing<[u8; 4096]>,
    _no_send: PhantomData<*mut ()>,
}

pub struct ValidatedEffect {
    pub effect_root: [u8; 32],
    pub receipt_root: [u8; 32],
    pub epoch: u64,
}

pub fn process_envelope(slot: EphemeralEnvelope) -> Result<ValidatedEffect, PalError> {
    let view = parse_in_place(slot.data.as_ref())?;
    let effect = validate_effect_view(view)?;
    Ok(effect.to_commitment())
}
```

The function consumes `slot` by value. Borrow-only entrypoints are insufficient
for the production invariant because the caller would retain lifetime control.

---

## §14.5 — WAL and Replay Boundary

The hosted replay scaffold may persist canonical raw test fixtures only under an
explicit non-production feature. This exists to generate proof vectors and
cross-ISA replay evidence.

A zero-persistence production WAL may contain only:

- epoch number,
- admitted scalar commitments,
- receipt root,
- EFB root,
- validation failure class without payload bytes,
- blind audit event identifiers,
- `ShredCommitment` identifiers,
- attestation status summaries without raw quotes unless separately approved by
  the compliance profile.

It MUST NOT contain `raw_txs`, raw envelope bytes, peer metadata, receipt bodies,
or graph topology.

---

## §14.6 — Hardware Profiles

### Global Standard profile

The Global Standard profile implements the software zero-persistence invariants:
non-serializable ephemeral types, admission-path no-copy discipline, production
WAL schema lock, blind audit events, and CI gates.

### Sovereign Hardened profile

The Sovereign Hardened profile adds hardware isolation:

- DPU/SmartNIC admission enclave receives raw packets before host memory.
- The DPU emits only scalar commitments or bounded admission records over PCIe.
- Host DMA is restricted by IOMMU policy to commitment-sized regions.
- Host RAM uses confidential-memory technology such as AMD SEV-SNP, Intel TDX,
  or ARM CCA where available.
- DPU firmware, admission code, and host policy are attested and pinned to the
  deployment profile.
- Tier 1/Tier 2 storage keys are generated and destroyed inside an HSM/TPM or
  sovereign-equivalent boundary.

These controls are deployment-profile requirements. They are not implied by the
base software scaffold.

---

## §14.7 — CI and Verification Gates

Before `zero-persistence` can be treated as implemented, the repository must add
and keep green the following gates:

```bash
cargo check -p qash-pal --no-default-features --features zero-persistence
cargo test -p qash-pal --features zero-persistence admission_no_raw_persistence
cargo test -p qash-pal --features zero-persistence production_wal_rejects_payload_bytes
cargo test -p qash-pal --features zero-persistence ephemeral_error_redaction
cargo test -p qash-consensus public_transcript_contains_no_graph_fields
```

A Kani or equivalent model-checking harness should prove that the production
admission path does not allocate, does not store raw envelope pointers in global
state, and cannot call the WAL raw-fixture writer.

Static tripwires are allowed as defense in depth but are not sufficient alone:

```bash
rg "Vec<u8>|\.clone\(\)|\.to_vec\(\)|Serialize|Debug|Display" crates/pal/src/admission
rg "payload|raw_tx|peer_ip|socket_addr" crates/pal/src/audit crates/pal/src/logging
```

Any intentional hit must be reviewed and justified in the PR body.

---

## §14.8 — Claims Boundary

Allowed technical claim after implementation and evidence:

> QASH zero-persistence production mode has no admissible software path that
> persists, serializes, logs, or emits raw transaction-graph material; Domain A
> receives only validated scalar effects and public roots.

Allowed Sovereign Hardened claim after hardware evidence:

> Under the declared Sovereign Hardened threat model, raw graph material is
> confined to an attested admission boundary and is not present in host memory,
> host WALs, public transcripts, or Domain A state.

Disallowed generic claims without additional legal and deployment review:

- subpoena immune,
- impossible to observe,
- mathematically impossible to recover,
- kernel never sees any packet in all deployments,
- raw material never exists anywhere.

The engineering term for this profile is **Zero-Persistence Graph
Non-Publication**.
