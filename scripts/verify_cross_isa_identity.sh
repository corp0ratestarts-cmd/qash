#!/usr/bin/env bash
set -euo pipefail

VECTORS="tests/vectors/vectors.v1.json"
TARGETS=( "x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" "riscv64gc-unknown-linux-gnu" )

ref_out=""

for target in "${TARGETS[@]}"; do
  cargo build -p qash-vector-runner --release --no-default-features --target "$target"
  bin="target/${target}/release/qash-vector-runner"
  out="out.${target}.json"

  case "$target" in
    x86_64-unknown-linux-gnu)
      "$bin" --vectors "$VECTORS" --out "$out"
      ;;
    aarch64-unknown-linux-gnu)
      qemu-aarch64 -L /usr/aarch64-linux-gnu "$bin" --vectors "$VECTORS" --out "$out"
      ;;
    riscv64gc-unknown-linux-gnu)
      qemu-riscv64 -L /usr/riscv64-linux-gnu "$bin" --vectors "$VECTORS" --out "$out"
      ;;
  esac

  if [[ -z "$ref_out" ]]; then ref_out="$out"; continue; fi
  diff -u "$ref_out" "$out"
done
