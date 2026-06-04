# ADR-018: Production Networking — Clone Protocol Transport Gap

**Status:** Accepted — Post-V1 (NET-2 through NET-7)  
**Date:** 2026-06-03  
**Authors:** Protocol team  
**Replaces:** None  
**Related:** `GENESIS_CONSTANTS.toml [clone_protocol]`, `docs/spec/19_profile_taxonomy.md`, ADR-013 (Domain B backend boundary)

---

## Context

`GENESIS_CONSTANTS.toml [clone_protocol]` specifies six transport channels
(`QR_code`, `NFC`, `BLE`, `WiFi_Direct`, `LoRa`, `Ultrasonic`) with various
hardware-specific parameters. The PAL clone protocol module
(`crates/pal/src/clone/`) implements the interface contract and wire framing
but does not wire these to real hardware drivers.

This ADR records the gap between the interface-only v1.0 state and the production
networking target, defines the gap scope for each channel, and defers all
hardware integration to post-v1.

---

## Decision: Networking Implementation Scope

### D1 — v1.0 networking scope (interface-only)

The following networking components are **complete and active in v1.0**:

| Component | File | Status |
|-----------|------|--------|
| `CloneTransport` trait + `TransportError` | `clone/transport/mod.rs` | `✅ ACTIVE V1` |
| `ChunkFrame` wire format (VERSION/EPOCH/SIG header) | `clone/transport/frame.rs` | `✅ ACTIVE V1` |
| Ultrasonic FSK physical framing + CRC-16/CCITT | `clone/transport/ultrasonic.rs` | `✅ ACTIVE V1` |
| LZ4 chunk compression + decompression | `clone/compression.rs` | `✅ ACTIVE V1` |
| Bloom filter dedup (`ChunkRelayFilter`) | `clone/dedup.rs` | `✅ ACTIVE V1` |
| Cover traffic scheduler + dummy payload | `clone/cover_traffic.rs` | `✅ ACTIVE V1` |
| Store-and-forward relay buffer | `clone/relay.rs` | `✅ ACTIVE V1` |
| Emergency wipe signal | `clone/wipe.rs` | `✅ ACTIVE V1` |
| Clone package manifest + root pair | `clone/manifest.rs` | `✅ ACTIVE V1` |
| `NetTransport` trait + `publish_transcript_entry` | `net/mod.rs` + `net/tcp_transport.rs` | `✅ ACTIVE V1` |

The following transport stubs are **interface-only** — correct MTU/channel-name constants,
correct `CloneTransport` type signatures, but all `send()`/`receive()` return
`Err(TransportError::NotAvailable)`:

| Transport stub | File | Status |
|----------------|------|--------|
| `QrTransport` | `clone/transport/stubs.rs` | `⚠️ INTERFACE-ONLY` |
| `NfcTransport` | `clone/transport/stubs.rs` | `⚠️ INTERFACE-ONLY` |
| `BleTransport` | `clone/transport/stubs.rs` | `⚠️ INTERFACE-ONLY` |
| `WifiDirectTransport` | `clone/transport/stubs.rs` | `⚠️ INTERFACE-ONLY` |
| `LoRaTransport` | `clone/transport/stubs.rs` | `⚠️ INTERFACE-ONLY` |
| `UltrasonicTransport` | `clone/transport/stubs.rs` | `⚠️ INTERFACE-ONLY` |

### D2 — Proximity / distance bounding (post-v1)

`crates/pal/src/proximity/distance_bounding.rs` provides the Hancke-Kuhn distance
bounding protocol stub. Transport wiring is post-v1.

### D3 — Production networking gap (NET-2 through NET-7)

| Task | Description | Target |
|------|-------------|--------|
| NET-1 | This ADR | ✅ Done |
| NET-2 | QR chunked self-authenticated transport driver | Post-v1 |
| NET-3 | NFC direct APDU transport driver | Post-v1 |
| NET-4 | BLE dual-role (concurrent peripheral + central) transport driver | Post-v1 |
| NET-5 | WiFi Direct transport driver | Post-v1 |
| NET-6 | LoRa transport driver | Post-v1 |
| NET-7 | Ultrasonic FSK driver (carrier of last resort; framing done, PHY integration needed) | Post-v1 |

### D4 — Domain B confinement (normative)

All networking code is Domain B. `CloneTransport::send()` and `CloneTransport::receive()`
MUST NOT accept or return raw `EpochState`. Only `CommitmentFrame` bytes (which are
derived from `PublicTranscript`, Class I visibility) may be transmitted over clone
channels.

`publish_transcript_entry()` in `net/mod.rs` is the sole authorized emission path for
public-channel data. This invariant is enforced by type constraints and is not affected
by the production networking gap.

### D5 — Faulty transport test harness

`crates/pal/src/net/faulty_transport.rs` provides a deterministic fault-injection harness
(drops, reorders, delays) used for protocol-level tests. This is complete and active in
v1.0. Production chaos testing of real transports is post-v1.

---

## Consequences

**Positive:**
- Wire format and framing are stable — NET-2..7 drivers can be integrated without
  changing the `CloneTransport` trait or `ChunkFrame` format.
- All auxiliary infrastructure (dedup, cover traffic, relay, wipe) is complete and
  requires no changes for production driver integration.
- Class I visibility boundary is enforced regardless of transport implementation status.

**Negative:**
- Real clone-protocol transfers between devices require post-v1 hardware driver work.
- PETN-CRCs integration (BLE-only, per genesis constants) is deferred until NET-4.

**Deferred:**
- All of NET-2 through NET-7 (see table in D3).
- Hancke-Kuhn distance bounding protocol wiring (post-v1, `proximity/` module).
- Platform-specific regulatory compliance for 2.4 GHz BLE / WiFi / LoRa emissions.
