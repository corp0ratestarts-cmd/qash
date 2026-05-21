#!/usr/bin/env bash
set -euo pipefail

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

CARGO_BIN="${CARGO:-cargo}"
ROOT_MARKER="CANONICAL_STATE_ROOT_3_EPOCHS"

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="${CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER:-aarch64-linux-gnu-gcc}"
export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER="${CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER:-riscv64-linux-gnu-gcc}"

cargo_args_for_target() {
  local target="$1"
  if [[ "${target}" == "x86_64-unknown-linux-gnu" ]]; then
    return 0
  fi
  printf '%s\n' --target "${target}"
}

capture_root() {
  local target="$1"
  local output
  local root
  local -a target_args
  mapfile -t target_args < <(cargo_args_for_target "${target}")

  output="$(
    "${CARGO_BIN}" test -p qash-consensus --no-default-features \
      "${target_args[@]}" \
      state_root_canonical_seq_print -- --nocapture 2>/dev/null
  )"

  root="$(printf '%s\n' "${output}" | grep "${ROOT_MARKER}" | head -1 || true)"
  if [[ -z "${root}" ]]; then
    echo "ERROR: ${ROOT_MARKER} not found for ${target}" >&2
    return 1
  fi
  printf '%s\n' "${root}"
}

run_consensus_suite() {
  local target="$1"
  local -a target_args
  mapfile -t target_args < <(cargo_args_for_target "${target}")

  "${CARGO_BIN}" test -p qash-consensus --no-default-features "${target_args[@]}"
  "${CARGO_BIN}" test -p qash-consensus --no-default-features \
    "${target_args[@]}" \
    --test v1_1_replay v1_1_corpus_matches_pinned
  "${CARGO_BIN}" test -p qash-consensus --no-default-features \
    "${target_args[@]}" \
    --test v1_2_sharded_replay v1_2_sharded_corpus_matches_pinned
}

reference_root="$(capture_root "x86_64-unknown-linux-gnu")"
echo "Native state root: ${reference_root}"

for target in "${TARGETS[@]}"; do
  echo "=== Verifying ${target} ==="
  target_root="$(capture_root "${target}")"
  echo "Target state root: ${target_root}"

  if [[ "${target_root}" != "${reference_root}" ]]; then
    echo "ERROR: state root divergence!" >&2
    echo "  native: ${reference_root}" >&2
    echo "  ${target}: ${target_root}" >&2
    exit 1
  fi

  run_consensus_suite "${target}"
done

echo "All configured targets produce identical canonical state roots and pass replay gates."
