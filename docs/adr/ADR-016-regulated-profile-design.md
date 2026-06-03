# ADR-016: QASH Regulated Profile Design

**Status:** Accepted  
**Date:** 2026-06-03  
**Authors:** Protocol team  
**Replaces:** None  
**Related:** ADR-015 (Pure QASH repo split), `docs/spec/19_profile_taxonomy.md`, `docs/spec/09_privacy_model.md`

---

## Context

ADR-015 established that the umbrella QASH repo (`corp0ratestarts-cmd/qash`) retains
Regulated Profile features while Pure QASH Core (`corp0ratestarts-cmd/pure-qash`) must
pass absence guards that reject all regulated-profile concepts.

This ADR records the design decisions for the Regulated Profile implementation in the
umbrella repo.

---

## Decision: Regulated Profile Architecture

### D1 — Feature flag isolation

Regulated Profile code is gated behind the `regulated` Cargo feature in `qash-pal`.
A binary compiled without `--features regulated` has no Class IV observer, no disclosure
key, and no lawful-basis machinery. This satisfies Rule PB-1 (no cross-profile contamination)
from `docs/spec/19_profile_taxonomy.md §19.2`.

```toml
# crates/pal/Cargo.toml
regulated = ["std"]
```

### D2 — Domain B confinement (normative)

All regulated-profile code lives in `crates/pal/src/regulated/`. Nothing in this module
may cross into Domain A (`crates/consensus/`). The `DisclosureKey` type is never passed
to any Domain A function.

Domain A remains identical whether or not `regulated` is enabled. The consensus core is
profile-unaware by construction.

### D3 — Class IV observer model

Class IV access requires three simultaneous gates (§P4a of `09_privacy_model.md`):

1. A genesis-authorised `DisclosureKey` (loaded from genesis constants or HSM).
2. A valid `DisclosureRequest` carrying a `LawfulBasis` (GDPR Art. 6/9 or national equivalent).
3. Epoch-scoped decryption — the key is valid only within `[activation_epoch, expiry_epoch)`.

### D4 — Non-retroactive disclosure (forward secrecy)

A `DisclosureKey` with `activation_epoch = T` cannot decrypt receipts from epoch `< T`.
After `max_offline_epochs` (12), the `epoch_seed` is destroyed and even the regulatory
authority cannot decrypt past-epoch receipts. This is the forward-secrecy property for
Class IV.

Implementation: `DisclosureKey::derive_epoch_key(epoch)` returns
`Err(EpochOutOfRange)` for `epoch < activation_epoch || epoch >= expiry_epoch`.

### D5 — Genesis constants

A `[regulated]` section is added to `GENESIS_CONSTANTS.toml` with `enabled = false`
and `disclosure_domains = []` as the default (Pure deployment, no Class IV).

A Regulated Profile genesis must set `enabled = true` and provide at least one
`[[regulated.disclosure_domain]]` entry with a genesis-committed `key_commitment`.

Changes to `[regulated]` after genesis require the full `[genesis-change-acknowledged]`
PR protocol.

### D6 — No production key material in genesis

The `key_commitment` field in genesis constants is a SHA3-256 commitment to the
private `DisclosureKey`. The private key itself is NOT stored in genesis constants or
any version-controlled file. It must be held in an HSM or secure enclave and loaded at
runtime via a separate key-management path.

### D7 — LawfulBasis is exhaustive

The `LawfulBasis` enum is exhaustive: `GdprArt6LegalObligation`, `GdprArt6PublicTask`,
`GdprArt9SubstantialPublicInterest`, and `NationalLawEquivalent`. A disclosure request
with no recognised lawful basis cannot be constructed.

---

## Implementation

The initial implementation is in `crates/pal/src/regulated/`:

| Module | Purpose |
|--------|--------|
| `mod.rs` | `ObserverClass` enum (with `ClassIV` under `regulated` feature), `LawfulBasis` |
| `disclosure.rs` | `DisclosureDomain`, `DisclosureKey`, `EpochDisclosureKey`, `DisclosureRequest`, `validate_disclosure_request()` |
| `receipt.rs` | `RegulatedReceiptDecrypt` — encrypt/decrypt with lawful-basis gate |

Tests (25 cases): epoch scope validation, key commitment determinism, roundtrip
encrypt/decrypt, tampered ciphertext rejection, blank requester rejection.

---

## Consequences

**Positive:**
- Regulated Profile is feature-flag isolated — no risk of contaminating Pure QASH builds.
- Domain A is unchanged; consensus determinism is unaffected.
- Forward secrecy is preserved for Class IV: non-retroactive by construction.
- The `LawfulBasis` type makes the legal basis mandatory at the call site.

**Negative:**
- Regulated Profile adds `std` dependency to the PAL feature surface.
- Production key management (HSM integration) is deferred — the current implementation
  uses in-memory key material; a production deployment needs an HSM or secure vault.

**Deferred:**
- HSM-backed key loading (post-v1).
- Multi-jurisdiction key management (multiple `DisclosureDomain` entries with rotation).
- Formal proof of Class IV non-retroactivity (theorem target in `proofs/privacy/`).
- Jurisdiction-specific compliance evidence docs (QASH-3.5+).

---

## Alternatives Considered

**Alternative A: Inline Class IV code in `crates/pal/src/receipt.rs`**  
Rejected: would contaminate the receipt module with regulated-profile concepts, making
it harder to audit the boundary and easier to accidentally enable Class IV in non-regulated builds.

**Alternative B: Separate crate (`qash-regulated`)**  
Deferred: would be cleaner long-term but adds workspace complexity at the current
implementation phase. The `regulated` feature gate in `qash-pal` achieves the same
isolation with less overhead until the feature matures.
