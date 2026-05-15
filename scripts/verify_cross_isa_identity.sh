#!/usr/bin/env bash
set -euo pipefail

VECTORS="${VECTORS:-tests/vectors/vectors.v1.json}"
if [[ -n "${TARGETS_OVERRIDE:-}" ]]; then
  # Space-separated override for local smoke tests, e.g.
  # TARGETS_OVERRIDE="x86_64-unknown-linux-gnu".
  read -r -a TARGETS <<< "${TARGETS_OVERRIDE}"
else
  TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "riscv64gc-unknown-linux-gnu"
  )
fi
REFERENCE=""

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="${CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER:-aarch64-linux-gnu-gcc}"
export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER="${CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER:-riscv64-linux-gnu-gcc}"

run_target() {
  local target="$1"
  local bin="target/${target}/release/qash-vector-runner"
  local out="out.${target}.json"

  ${CARGO:-cargo} build -p qash-vector-runner --release --target "${target}"

  case "${target}" in
    x86_64-unknown-linux-gnu)
      "${bin}" --vectors "${VECTORS}" --out "${out}"
      REFERENCE="${out}"
      ;;
    aarch64-unknown-linux-gnu)
      qemu-aarch64 -L /usr/aarch64-linux-gnu "${bin}" --vectors "${VECTORS}" --out "${out}"
      ;;
    riscv64gc-unknown-linux-gnu)
      qemu-riscv64 -L /usr/riscv64-linux-gnu "${bin}" --vectors "${VECTORS}" --out "${out}"
      ;;
    *)
      echo "unsupported target: ${target}" >&2
      return 2
      ;;
  esac

  if [[ "${target}" != "x86_64-unknown-linux-gnu" ]]; then
    echo "Comparing ${target} against ${REFERENCE}..."
    diff -u "${REFERENCE}" "${out}" || {
      echo "DETERMINISM FAILURE: ${target} diverges from x86_64" >&2
      return 1
    }
  fi
}

for target in "${TARGETS[@]}"; do
  run_target "${target}"
done

echo "All targets produce identical vector outputs."
