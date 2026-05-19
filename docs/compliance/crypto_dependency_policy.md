# Crypto Dependency Bump Policy

Policy: no cryptographic dependency version bumps may be merged unless the PR
includes a refreshed crypto conformance report artifact from the
`crypto-conformance` CI job (Domain B lane).

Required evidence:
- Passing `crypto-conformance` job output for the candidate commit.
- Updated `tests/vectors/crypto/SHA256SUMS` if vector files changed.
- Updated `docs/compliance/crypto_conformance_matrix.md` when scope/source changes.
