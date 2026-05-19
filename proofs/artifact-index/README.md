# Proof Artifact Index

This directory contains committed proof-hash manifests produced by CI on every
push to `main`. Each file is named `proof-hashes-<commit-sha>.txt` and records
the SHA-256 of every compiled `.vo` proof object alongside the Coq version used.

## Purpose

GitHub Actions artifacts expire after 365 days. This directory provides a
**permanent, git-history-backed** record that auditors can use to verify proof
objects long after the original CI run is gone.

## File format

Each `proof-hashes-<sha>.txt` has the structure:

```
# coq-version: Coq <version>
<sha256>  ./<path/to/file.vo>
<sha256>  ./<path/to/file.vo>
...
```

## How to use

1. Find the commit SHA you want to audit (e.g., `git rev-parse v1.0-reference`).
2. Open `proofs/artifact-index/proof-hashes-<sha>.txt`.
3. Note the `# coq-version:` header and install that exact Coq version.
4. Run `scripts/capture_proof_hashes.sh` locally (it compiles the proofs and
   prints the hash manifest).
5. Compare the output against the committed manifest line by line.

Hashes will only match when the **exact same Coq version** is used. The
version string is recorded in the `# coq-version:` header of each manifest.

## Adding an entry (release procedure)

After creating a release tag, the CI `proofs` job running on `main` will
automatically commit the manifest for that commit. For manually-triggered
captures, run:

```sh
./scripts/capture_proof_hashes.sh | \
  tee proofs/artifact-index/proof-hashes-$(git rev-parse HEAD).txt
git add proofs/artifact-index/
git commit -m "ci: record proof hashes for $(git rev-parse --short HEAD)"
```
