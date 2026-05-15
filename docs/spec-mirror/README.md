# Non-Normative Spec Mirrors

The files in this directory are convenience mirrors and implementation-facing
views of the normative PDF in `spec/pdf/QASH_Spec_v1.0.pdf`.

## Mirror rules

- Every mirrored section must name the normative PDF section and page range.
- Mirrors may not introduce new protocol requirements.
- Any text that goes beyond the PDF must be marked `NOT IN PDF` and linked to
  an ADR or erratum.
- If a mirror conflicts with the PDF, the PDF wins unless an accepted erratum
  explicitly changes the PDF.

Existing documents under `docs/spec/` are treated as pre-existing engineering
specifications that must be migrated or cross-linked into this mirror hierarchy
before genesis lock.
