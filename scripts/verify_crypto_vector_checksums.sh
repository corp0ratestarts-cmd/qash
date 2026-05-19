#!/usr/bin/env bash
set -euo pipefail

expected_file="tests/vectors/crypto/SHA256SUMS"
if [[ ! -f "$expected_file" ]]; then
  echo "missing checksum manifest: $expected_file" >&2
  exit 1
fi

( cd tests/vectors/crypto && sha256sum -c SHA256SUMS )
