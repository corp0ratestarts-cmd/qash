# Proof Reproducibility

## Audit trail — two layers

| Layer | Location | Lifespan |
|-------|----------|----------|
| **Committed manifest** | `proofs/artifact-index/proof-hashes-<sha>.txt` | Permanent (git history) |
| **CI artifact** | `proof-objects-<sha>` on GitHub Actions | 365 days |

On every push to `main` the `proofs` CI job:
1. Compiles all active Coq proofs with the pinned system Coq version.
2. Records a hash manifest (`# coq-version:` header + SHA-256 of each `.vo`).
3. **Commits the manifest** to `proofs/artifact-index/proof-hashes-<sha>.txt`
   — permanently captured in git history at the source commit it corresponds to.
4. Uploads the same manifest as a GitHub Actions artifact (365-day retention).

This means auditors can verify proof objects for any `main` commit long after
the CI artifact expires: the committed manifest stays in git forever.

---

## Verifying proofs for a specific commit

```sh
# 1. Find the commit SHA (e.g. for a release tag):
SHA=$(git rev-parse v1.0-reference)

# 2. Read the committed manifest:
cat proofs/artifact-index/proof-hashes-${SHA}.txt
# First line:  # coq-version: Coq <version>
# Remaining:   <sha256>  ./<path/to/file.vo>

# 3. Install the exact Coq version shown in the header.
#    Via opam:
opam install coq=<version>

# 4. Compile and compare locally:
./scripts/capture_proof_hashes.sh
```

The local output should match the committed manifest line-for-line.
Hashes differ **only if** the Coq version differs — they are not reproducible
across Coq versions because `.vo` files embed version metadata.

---

## `scripts/capture_proof_hashes.sh`

A standalone shell script that replicates the CI compilation sequence locally:

```sh
# Print manifest to stdout:
./scripts/capture_proof_hashes.sh

# Capture for current HEAD (e.g. at a release tag):
./scripts/capture_proof_hashes.sh \
  | tee proofs/artifact-index/proof-hashes-$(git rev-parse HEAD).txt
```

The script compiles proofs in the same tier order used by CI and exits non-zero
if any proof fails to compile.

---

## Notes on reproducibility

Coq `.vo` files are **not** byte-for-byte identical across Coq versions — the
binary format includes version metadata. Hashes will only match if you use the
exact same Coq version recorded in the `# coq-version:` header.

The CI currently installs Coq via `apt-get` on Ubuntu latest. To pin to an
exact version, install from `opam` with a version constraint:

```sh
opam install coq=<version>
```

A Nix flake with a pinned Coq version is a planned improvement
(see `PROJECT_STATUS.md` Phase 3).

---

## No `Admitted` policy

The CI `proofs` job runs a Python script that scans all active proof files
(excluding `_wip/`) for bare `Admitted` or `admit` markers after stripping
Coq comments. Any such marker fails CI immediately. This guarantees that
all theorems listed as `PROVED` in `COVERAGE.md` have complete machine-checked
proofs, not axiomatised stubs.
