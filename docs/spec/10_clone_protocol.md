# QASH Clone Protocol — §10

**Status:** NORMATIVE  
**Spec version:** v1.2  
**Implements:** `GENESIS_CONSTANTS.toml [clone_protocol]`

---

## §10.0 — Purpose and scope

The clone protocol enables QASH validators to synchronize state and submit
transactions in environments where the primary consensus mesh is unavailable:
radio-jammed zones, air-gapped facilities, bandwidth-constrained deployments,
and offline mobile operation.

The clone protocol is a **Domain B** artifact. Raw transaction envelopes and
clone chunks are internal Domain B data. The public Domain A surface (`PublicTranscript`)
is never extended by clone protocol artifacts; only the resulting `state_root` and
`receipt_root` after application are Domain A visible.

---

## §10.1 — Transport layer abstraction

The clone protocol is transport-agnostic. Any admitted transport satisfies the
same interface contract; the consensus layer never observes which transport
delivered a given chunk. The genesis-admitted transport set is:

| ID | Transport | Typical range | Use case |
|----|-----------|--------------|----------|
| `QR_code` | Visual QR scan | Contact | Air-gapped handoff, paper backup |
| `NFC` | ISO 14443 APDU | ≤ 10 cm | Tap-to-sync mobile validators |
| `BLE` | Bluetooth Low Energy 5.x | ≤ 100 m | Standard wireless sync |
| `WiFi_Direct` | IEEE 802.11 P2P | ≤ 200 m | High-throughput local sync |
| `LoRa` | LoRa/LoRaWAN PHY | ≤ 15 km | Long-range, low-power rural deployment |
| `Ultrasonic` | FSK 24 kHz carrier | ≤ 10 m | Radio-jammed / Faraday-cage environments |

Transport bonding (simultaneous transmission across multiple transports for
redundancy or throughput aggregation) is a Domain B concern and is not specified
at this layer. The protocol processes the first valid received copy of each chunk.

---

## §10.2 — Ultrasonic transport (§10.2 normative; optional/experimental)

> **Implementation note:** The ultrasonic transport is **optional and
> experimental**.  Acoustic hardware support is highly platform-fragmented:
> mobile SoCs, embedded targets, and desktop hosts vary widely in whether they
> expose a 24 kHz piezoelectric path, and OS audio stacks impose unpredictable
> latency.  Platform integrators MUST gracefully handle transport unavailability
> (`TransportError::NotAvailable`) and SHOULD prefer higher-priority transports
> (§10.3) whenever available.  The `UltrasonicTransport` PAL stub is present for
> completeness; production deployment requires explicit platform HAL integration
> and hardware qualification testing.

The ultrasonic transport channel is admitted at genesis for deployment in
radio-frequency-denied environments (Faraday cages, military-grade RF jamming,
hospital RF-quiet zones, underground facilities).

**Physical parameters:**
```
Carrier frequency:   24 kHz (above human audible range ≥ 20 kHz threshold)
Modulation:          Binary FSK
  Mark frequency:    24.0 kHz  (bit 1)
  Space frequency:   24.5 kHz  (bit 0)
Symbol rate:         ≤ 1200 baud (hardware-dependent; 300/600/1200 admitted)
Effective data rate: ~100–150 bytes/sec at 1200 baud after framing overhead
Max range:           ~10 m in open air; reduced in acoustically absorptive environments
Hardware:            Standard piezoelectric transducer + ADC/DAC; no specialized RF hardware
```

**Framing:**
```
[SYNC 4 bytes][LEN 2 bytes LE][PAYLOAD ≤ 255 bytes][CRC16 2 bytes]
SYNC = 0x55 0x55 0xAA 0x55   (preamble for FSK synchronization)
CRC16 = CRC-CCITT over LEN || PAYLOAD
```

Ultrasonic chunks use the same Domain B chunk authentication scheme as all other
transports (see §10.4). The physical channel provides framing only; confidentiality
and integrity guarantees come from the chunk signing layer.

**Operational notes:**
- Ambient noise above 80 dB SPL degrades effective range.
- Multiple simultaneous ultrasonic emitters in the same space cause interference;
  the protocol does not specify TDMA arbitration — implementations SHOULD use
  exponential-backoff retry.
- Ultrasonic is the carrier of last resort in the transport priority order
  (§10.3); implementations SHOULD prefer higher-bandwidth transports when available.

---

## §10.3 — Transport selection and priority

When multiple transports are available, the Domain B implementation SHOULD prefer
transports in this order (highest throughput / lowest latency first):

```
1. WiFi_Direct   (highest throughput)
2. BLE           (dual-mode: concurrent peripheral + central)
3. NFC           (contact; zero configuration)
4. LoRa          (long range; low throughput)
5. QR_code       (air-gap; human-mediated)
6. Ultrasonic    (RF-denied fallback; carrier of last resort)
```

Transport bonding (parallel use of multiple transports) and dynamic switching
are Domain B implementation choices and are not constrained by this spec.

---

## §10.4 — Dual-mode BLE operation

BLE operates in **dual mode**: the validator simultaneously acts as a peripheral
(advertising, accepting connections) and as a central (scanning, initiating
connections). This removes the master/slave asymmetry that would otherwise require
explicit role negotiation.

**Implications:**
- Any two validators within BLE range can exchange clone chunks without prior
  pairing or role assignment.
- Relay validators (§10.5) can accept incoming chunks (peripheral) and immediately
  forward them (central) without mode-switching delay.
- BLE advertisement payloads MUST NOT contain validator identity; they contain
  only the epoch counter and a random ephemeral session token to prevent
  cross-epoch linkage by passive BLE scanners.

---

## §10.5 — Store-and-forward relay

When `store_and_forward = true` (genesis-set), intermediate validators MAY buffer
clone chunks for peers that are temporarily unreachable and forward them when
contact is re-established.

**Invariants:**
- A relay node MUST NOT apply buffered chunks to its own state before forwarding;
  application is the responsibility of the destination validator.
- Buffered chunks MUST be dropped after `max_offline_epochs` epochs have elapsed
  since the chunk's embedded epoch counter. Stale chunks are inadmissible.
- Buffer contents are Domain B data and are never included in `PublicTranscript`.

---

## §10.6 — Cover traffic and timing obfuscation

When `cover_traffic = true` (genesis-set), validators MUST emit dummy clone-protocol
packets at a constant rate on all active transports, indistinguishable in size and
timing from real chunk traffic.

**Purpose:** A passive observer watching BLE/WiFi-Direct/LoRa traffic cannot
determine whether a validator is actively syncing state or is idle. Timing
side-channels that would allow graph reconstruction (validator A synced with
validator B at time T) are suppressed.

**Implementation constraint:** Cover packets MUST be:
- The same byte length as real chunks (padded to maximum chunk size).
- Signed with a per-epoch ephemeral key distinct from the validator's consensus key.
- Indistinguishable from real chunks to a passive observer without the epoch
  disclosure key.

Cover traffic is a Domain B concern; its cryptographic construction is deferred
to the Domain B blinding spec (§P8 of `09_privacy_model.md`).

---

## §10.7 — Binary packet format and compression

Clone chunk payloads are compressed with LZ4 (`packet_compression = "LZ4"`) before
chunk authentication. LZ4 is chosen for:
- Deterministic compression output given identical input (required for Domain A
  state-root reproducibility after decompression).
- Negligible decompression latency on constrained hardware (ESP32, Cortex-M).
- No entropy source required (unlike stream ciphers in compression mode).

**Chunk wire format (Domain B):**
```
[VERSION u8][EPOCH u64 LE][CHUNK_IDX u16 LE][CHUNK_TOTAL u16 LE]
[COMPRESSED_LEN u16 LE][PAYLOAD ≤ 4096 bytes LZ4-compressed]
[SIG 2420 bytes Dilithium5]
```

- `VERSION` = 0x12 for v1.2.
- Decompressed payload MUST NOT exceed `4096 × CHUNK_TOTAL` bytes; oversized
  decompressed output is inadmissible (prevents decompression-bomb attacks).
- `SIG` covers all preceding bytes in the chunk (VERSION through PAYLOAD).

---

## §10.8 — Bloom filter deduplication

When `bloom_filter_dedup = true` (genesis-set), relay nodes maintain a per-epoch
Bloom filter of received chunk IDs to suppress redundant retransmission.

**Parameters (Domain B recommendation):**
```
Filter size:   8192 bits (1 KB)
Hash functions: 7 (SHA3-256 with index suffix)
Expected FPR:  < 0.01 at 1000 chunks/epoch
Reset:         At each new epoch (filter is epoch-scoped)
```

The Bloom filter is a Domain B performance optimization. A false positive
(duplicate suppression of a unique chunk) is recoverable: the destination
validator will re-request the missing chunk via explicit NACK. The filter MUST
NOT be used to gate Domain A admission — chunk authentication and epoch validity
checks govern admissibility.

---

## §10.9 — Emergency wipe

When `emergency_wipe_signal = true` (genesis-set), a validator holding a genesis-admitted
validator key MAY broadcast a signed emergency wipe command. On receipt, an
implementation SHOULD:

1. Immediately halt all in-progress Domain A transitions (entering absorbing halt
   state `0x06 HaltFlagSet`).
2. Cryptographically erase all locally cached Domain B secrets (private keys,
   TEE-bound material, buffered clone chunks).
3. Preserve the public `PublicTranscript` log (state roots, receipt roots) for
   audit purposes.

**Normative constraint:** The wipe command MUST be authenticated with a validator
consensus key and include the current epoch counter; replayed wipe commands from
prior epochs are inadmissible.

**Purpose:** Designed for mobile validators in high-risk physical environments
(seizure risk, device compromise). The halt-then-wipe order ensures no partial
state is exposed: the Domain A state machine reaches a clean absorbing halt before
any erasure operation.

---

## §10.10 — Offline operation limits

```
max_offline_epochs = 12   (genesis-set; ~6 seconds at 500 ms/epoch)
```

A validator that has been offline for more than `max_offline_epochs` epochs
MUST NOT apply accumulated clone chunks to advance its local state directly.
It must instead re-synchronize via the full Domain A state-root verification
path before resuming normal operation.

This limit exists to bound the maximum Lyapunov window divergence that can
accumulate while offline, ensuring that re-joining validators cannot destabilize
the convergence invariant.

---

## §10.11 — Security properties

| Property | Mechanism | Status |
|----------|-----------|--------|
| Chunk authenticity | Dilithium5 signature over full chunk | IMPLEMENTED (genesis-admitted) |
| Replay resistance | Epoch counter in chunk header; stale epoch → inadmissible | IMPLEMENTED |
| Observer unlinkability | Cover traffic; ephemeral BLE advertisement tokens | GENESIS-SPECIFIED; Domain B impl required |
| Graph non-publication | Clone chunks never appear in `PublicTranscript` | ENFORCED by `PublicTranscript` type boundary |
| Decompression safety | Max decompressed size bound in chunk format | SPECIFIED; Domain B impl required |
| Emergency key erasure | Wipe command + absorbing halt | GENESIS-SPECIFIED; Domain B impl required |

---

## §10.12 — Out of scope (deferred)

- TDMA arbitration for ultrasonic multi-emitter environments
- Transport bonding throughput aggregation
- Clone protocol authentication key rotation schedule
- Domain B blinding of clone chunk payloads (deferred to Domain B blinding spec)
- LoRa network coding / FEC specification
- WiFi-Direct group owner negotiation
