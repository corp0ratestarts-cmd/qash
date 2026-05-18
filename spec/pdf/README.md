# Normative PDF Specification

This directory is reserved for the immutable, versioned PDF specification that
anchors QASH protocol intent.

## Required artifact

- `QASH_Spec_v1.0.pdf` — **normative source of truth** for QASH v1.0.

Until the PDF is checked in, every quote and page reference in
`docs/traceability.md`, `docs/errata/`, and `docs/adr/` is treated as
**provisional** and must be verified against the committed PDF before genesis
lock.

## Authority rule

1. The checked-in PDF defines intended behavior.
2. `docs/spec-mirror/` contains non-normative, anchored convenience mirrors.
3. `docs/errata/` contains normative corrections or clarifications to the PDF.
4. `docs/adr/` contains engineering decisions and implementation constraints.
5. Code that cannot be traced through `docs/traceability.md` is not considered
   spec-covered.

## Genesis hash status

The current repository state is **pre-lock** because `QASH_Spec_v1.0.pdf` is
not committed. `GENESIS_CONSTANTS.toml` therefore marks `genesis_hash` as
provisional and not deployment-authoritative.

The exact pre-lock artifact set used by `scripts/verify_genesis_hash.sh` is
documented in `spec/genesis-artifacts.txt`. When the PDF is committed for
genesis lock, add `spec/pdf/QASH_Spec_v1.0.pdf` to that manifest, re-verify every
provisional quote and page reference in `docs/traceability.md`, `docs/errata/`,
and `docs/adr/`, then recompute and update the recorded hash.
