# Pure QASH Claim Boundary

**Status:** Normative  
**Scope:** All public-facing claims about Pure QASH properties.

All claims made about Pure QASH must appear in or be derived from this register.
No claim may be made outside this register without a corresponding PR updating it.

---

## Allowed claims (bounded and verifiable)

```
Pure QASH publishes no transaction graph.
Pure QASH public transcript is root-only: (epoch, state_root, receipt_root, efb_root, halt_flag).
Pure QASH persists no user graph material in production mode.
Pure QASH has no genesis-authorised disclosure key.
Pure QASH has no priority fee or fee auction.
Pure QASH uses deterministic constitutional scarcity (decaying issuance, fixed tail).
Pure QASH burns 100% of transaction fees.
Pure QASH burns 100% of slash amounts.
Pure QASH eliminates endogenous Domain-A MEV by construction.
Pure QASH certification evidence is blind (proves control behavior, not user behavior).
Pure QASH has no Class IV (regulatory authority) observer class.
Pure QASH has no monetary governance.
```

---

## Non-claims (explicit exclusions — do not assert these)

```
Does NOT claim endpoint-compromise immunity.
  (Cryptographic privacy holds only if keys are not extracted.)

Does NOT claim transport-layer anonymity.
  (Unless a separate transport privacy profile is implemented and documented.)

Does NOT claim to eliminate off-protocol exchange arbitrage.
  (Economic layer outside consensus is out of scope.)

Does NOT claim external certification by any standards body.

Does NOT include lawful disclosure keys.

Does NOT support regulated receipt disclosure.

Does NOT claim "immune to all metadata attacks."
  (Global passive adversary timing correlation is not solved by graph non-publication alone.)

Does NOT claim "eliminates all MEV everywhere."
  (Off-protocol, exchange-layer, and bridge MEV are out of scope.)
```

---

## Claim addition process

Adding a new allowed claim requires:
1. A PR to this file with the claim text
2. A corresponding proof, test, or evidence entry in `docs/status/` or `proofs/`
3. Verification that the claim does not contradict any non-claim above

A claim with no supporting evidence entry is inadmissible.
