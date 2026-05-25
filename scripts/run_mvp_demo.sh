#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-artifacts/mvp-demo/local-run}"
NODE_A="$ROOT_DIR/node-a"
NODE_B="$ROOT_DIR/node-b"
LOG="$ROOT_DIR/run.log"

rm -rf "$ROOT_DIR"
mkdir -p "$ROOT_DIR"

run() {
  echo "+ $*" | tee -a "$LOG"
  "$@" 2>&1 | tee -a "$LOG"
}

run cargo run -- demo init --dir "$NODE_A"
run cargo run -- demo issue-receipt --dir "$NODE_A" --epoch 1 --nonce-hex 0101010101010101010101010101010101010101010101010101010101010101 --body "offline pump-station door alarm"
run cargo run -- demo issue-receipt --dir "$NODE_A" --epoch 1 --nonce-hex 0202020202020202020202020202020202020202020202020202020202020202 --body "offline generator tamper event"
run cargo run -- demo sync --dir "$NODE_A" --peer-dir "$NODE_B" --out "$ROOT_DIR/public_commitments.bin"
run cargo run -- demo replay --dir "$NODE_A"

RECEIPT_ID=$(grep '^receipt_id:' "$LOG" | head -n1 | awk '{print $2}')
if [[ -z "$RECEIPT_ID" ]]; then
  echo "failed to extract receipt_id" >&2
  exit 1
fi

run cargo run -- demo disclose --dir "$NODE_A" --receipt-id "$RECEIPT_ID" --out "$ROOT_DIR/disclosure.bin"

if grep -a -q "offline pump-station door alarm" "$ROOT_DIR/public_commitments.bin"; then
  echo "public commitment export leaked private receipt body" >&2
  exit 1
fi

if ! grep -a -q "offline pump-station door alarm" "$ROOT_DIR/disclosure.bin"; then
  echo "selected disclosure did not contain selected receipt body" >&2
  exit 1
fi

if grep -a -q "offline generator tamper event" "$ROOT_DIR/disclosure.bin"; then
  echo "selected disclosure leaked unrelated receipt body" >&2
  exit 1
fi

cat > "$ROOT_DIR/manifest.txt" <<MANIFEST
QASH MVP demo artifact manifest
workspace: $ROOT_DIR
node_a: $NODE_A
node_b: $NODE_B
public_commitments: $ROOT_DIR/public_commitments.bin
disclosure: $ROOT_DIR/disclosure.bin
log: $LOG
claim: local Domain B offline incident receipt commit demonstrator only
MANIFEST

echo "MVP demo artifact bundle written to $ROOT_DIR"
