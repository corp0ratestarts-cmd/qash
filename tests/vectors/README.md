# QASH Golden Vectors

Golden vectors are executable spec artifacts. They bind PDF requirements,
errata, ADR decisions, code, and cross-ISA determinism checks.

## Required vector classes before genesis lock

1. Fixed-point arithmetic vectors derived from PDF §2.4.
2. Leaf-index vectors derived from PDF §3.2.
3. State-root vectors after ADR-003 defines full state encoding.
4. Halt-path vectors after ERR-001 and ADR-004 are resolved.
5. Cross-ISA replay vectors that compare final and intermediate state roots
   byte-for-byte.

## Shape

Multi-epoch vectors must include expected outputs for every epoch, not only the
final epoch. Intermediate roots make masking bugs visible when later transitions
happen to converge on the same final output.

## Current scaffold

`vectors.v1.json` is a code-derived manifest used by `qash-vector-runner` until
ADR-003 defines the full state encoding. The cross-ISA gate compares runner
outputs across targets; it does not yet claim that those state-root bytes are
PDF-derived golden roots.

## Rule

No traceability row may be marked ✅ unless it links to a vector or test that is
run in CI.
