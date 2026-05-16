# QASH ABCR Handshake Protocol

**Status:** Normative (Domain B)
**Scope:** Attestation-Backed Challenge-Response (ABCR) admission channel.
Zero consensus footprint — no Domain A code changes required.

---

## §H0 — Transport Security

Before any ABCR protocol steps execute, a forward-secret session must be
established between the two parties. QASH uses a post-quantum variant of the
Noise Protocol Framework (Perrin, 2018) as the underlying handshake pattern:

```
QASH ABCR Handshake transport:
  Noise_XX_Kyber768_SHA3-256_BLAKE3

Where:
  XX        = mutual authentication (both parties contribute ephemeral keys)
  Kyber768  = post-quantum KEM (NIST PQC / FIPS 203)
  SHA3-256  = hash function (already in QASH cascade — NIST FIPS 202)
  BLAKE3    = chaining hash (already in QASH cascade)
```

**Rationale:** The XX pattern provides mutual authentication without
pre-existing shared state; both parties prove possession of their long-term
keys during the handshake. Kyber768 replaces the Noise default of X25519 to
satisfy post-quantum security requirements consistent with the rest of the
QASH crypto cascade.

The ABCR attestation quote and `blinding_health_proof` ride as handshake
**message payloads** after the Noise session is established. This cleanly
separates transport security from QASH-specific attestation logic.

---

## §H1 — ABCR Protocol (normative)

The ABCR handshake solves the "deliver to compromised device" problem that a
simple dust-echo cannot: a compromised device can echo dust back. ABCR forces
attestation at handshake time, triggering absorbing halt on the recipient
device before any response is sent if integrity fails.

### Step 1 — Challenge (Sender → Recipient)

Sent as a valueless message via the Noise-secured session. Never broadcast to
the public Domain A mesh.

```
challenge = {
  nonce:           [u8; 32],    // epoch-bound, sender-generated (CSPRNG in Domain B)
  domain:          "QASH:HANDSHAKE",  // domain-separated from all consensus domains
  epoch_binding:   u64,         // current epoch number — binds challenge to epoch
  max_value_bound: u128,        // optional policy hint (0 = no limit)
}
```

### Step 2 — Attested Response (Recipient → Sender)

Generated entirely inside TEE/OEM domain before transmission.

```
response = {
  signature:            PQ_SIG,     // Signs H(nonce ∥ domain ∥ epoch_seed ∥ blinding_hash)
                                    // using Dilithium5 (primary) per GENESIS_CONSTANTS
  attestation_quote:    bytes,      // Fresh TPM/TEE quote chained to genesis_config.attestation_root
  blinding_health_proof: [u8; 32], // Domain B blinding integrity hash.
                                    // Computed by PAL::Attest over the current
                                    // obfuscation-cascade state; never derived
                                    // from Domain A values. Threshold check is
                                    // policy-configurable but never zero.
  epoch_seed_binding:   [u8; 32],  // Proves response is epoch-bound via local cascade state
}
```

**Absorbing halt trigger:** If attestation verification or blinding health check
fails during response generation, the device triggers `Halt::absorbing_reset()`
and sends no response. The challenge times out on the sender side.

### Step 3 — Verification (Sender)

All of the following must hold; any failure aborts the handshake:

1. `signature` is valid against the recipient's known public identifier
2. `attestation_quote` chains to `genesis_config.attestation_root`
3. `blinding_health_proof` meets the configured minimum threshold
4. `nonce` matches the sent challenge
5. `epoch_binding` matches the sender's current epoch
6. `epoch_seed_binding` is consistent with the local cascade state

On pass: proceed with transfer. On fail: abort — no dust sent, no graph leaked.

---

## §H2 — Security Properties

| Property | Mechanism |
|----------|-----------|
| Zero consensus footprint | Valueless handshake, peer-to-peer only, no broadcast to Domain A |
| Offline/async compatible | QR/NFC/BLE transport; response can be cached within epoch |
| Replay resistance | Domain-separated (`QASH:HANDSHAKE`), epoch-bound nonce |
| Forward secrecy | Noise_XX ephemeral keys; compromise of long-term keys does not reveal past sessions |
| Attested liveness | Quote freshness verified against epoch — stale quotes rejected |
| Client-policy configurable | `max_value_bound` allows per-transfer policy enforcement |

---

## §H3 — Threat Model

| Threat | ABCR Response |
|--------|---------------|
| Compromised device (key extracted) | Attestation quote fails genesis root check → absorbing halt before response |
| Replay of captured response | Epoch-bound nonce — replays from prior epochs are rejected |
| MITM on transport | Noise_XX mutual auth — both parties authenticate; MITM cannot forge |
| Simple dust-echo attack | ABCR requires fresh TEE attestation; echo without attestation fails Step 3 |
| Offline timing correlation | Response cacheable within epoch; no fixed timing signature |

---

## §H4 — Integration Layer

| Component | Location | Notes |
|-----------|----------|-------|
| Noise_XX_Kyber768 session | Domain B / PAL `Net` trait impl | OEM-specific; not in Domain A |
| Challenge generation | Domain B / wallet/OEM layer | `nonce` uses CSPRNG from Domain B entropy |
| Attestation quote | Domain B / PAL `Attest` trait | TEE/TPM; OEM-specific |
| Blinding health check | Domain B / PAL blinding layer | Deferred; see `09_privacy_model.md §P8` |
| Absorbing halt on failure | Domain A boundary / PAL `Halt` trait | `Halt::absorbing_reset()` from `qash-pal` |

No `qash-consensus` (Domain A) code is modified by this protocol. The ABCR
channel is purely a Domain B admission and transport concern.

---

## §H5 — Implementation Notes

- `Noise_XX_Kyber768_SHA3-256_BLAKE3` is not a published standard at time of
  writing. QASH implements it as a Noise extension following the Noise spec
  §9.3 (additional algorithms). The KEM interface uses Kyber768 encaps/decaps
  in place of the Noise DH primitive.
- The `PQ_SIG` in Step 2 uses Dilithium5 as specified in `GENESIS_CONSTANTS.toml`
  (`[crypto_cascade] primary = "Dilithium5"`).
- Epoch-binding is achieved by including `epoch_seed_t` (from `EpochState`) in
  the signed payload — this is a Domain A value read at the Domain B boundary;
  it is a read-only input and does not constitute cross-domain contamination.
