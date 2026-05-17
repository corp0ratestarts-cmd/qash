# Proof Reproducibility

Every CI run of the `proofs` job uploads two artifacts under the name
`proof-objects-<commit-sha>`:

| File | Contents |
|------|----------|
| `proof-coq-version.txt` | Output of `coqc --version` — the exact Coq build that compiled the proofs |
| `proof-hashes.txt` | SHA-256 of each `.vo` proof-object file, sorted by path |

These artifacts allow an independent party to verify that:
1. The proofs compile under a specific Coq version
2. The compiled proof objects match the expected hashes for a given source commit

---

## Reproducing locally

Install the same Coq version shown in `proof-coq-version.txt`, then:

```sh
# From the repository root
cd proofs

# Tier 1
coqc -Q . QASH util/list_inj.v
coqc -Q . QASH contractivity/lyapunov_stability.v

# Tier 2
for f in \
  concat_injective.v \
  contractivity/encode_injectivity.v \
  contractivity/tx_perturbation_0.v \
  contractivity/tx1_score_decrement.v \
  contractivity/lyapunov_grace_convergence.v \
  lyapunov_decrease.v \
  safety/absorbing_halt.v \
  integration/th8_composition.v \
  cascade/cascade_health_bounded.v \
  cascade/cascade_determinism.v \
  cascade/cascade_collision_resistance.v \
  blinding/blinding_non_interference.v \
  model/Model.v; do
  coqc -Q . QASH "$f"
done

# Generate your own hash manifest
find . -name "*.vo" | sort | xargs sha256sum
```

Compare the output against `proof-hashes.txt` from the CI artifact for the same commit.

---

## Notes on reproducibility

Coq `.vo` files are **not** byte-for-byte identical across Coq versions — the
binary format includes version metadata. Hashes will only match if you use the
exact same Coq version recorded in `proof-coq-version.txt`.

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
