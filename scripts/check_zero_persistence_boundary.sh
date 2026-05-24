#!/usr/bin/env bash
set -euo pipefail

# Guard the production zero-persistence PAL path. The hosted replay scaffold may
# still contain raw fixture handling, but admission and production WAL modules
# must remain commitment-only.

cargo check -p qash-pal --features zero-persistence --no-default-features
cargo test -p qash-pal --features zero-persistence --test zero_persistence

for path in crates/pal/src/admission.rs crates/pal/src/zero_wal.rs; do
  if rg -n 'raw_txs|raw_tx|payload|peer_ip|socket_addr|Vec<u8>|String|to_vec\(|clone\(' "$path"; then
    echo "zero-persistence boundary violation in $path" >&2
    exit 1
  fi
done

if rg -n 'Raw|Payload|Tx' crates/pal/src/zero_wal.rs; then
  echo "zero-persistence WAL must not expose raw/payload/tx variants" >&2
  exit 1
fi

echo "zero-persistence boundary gate passed"
