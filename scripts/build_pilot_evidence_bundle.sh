#!/usr/bin/env bash
# Build a pilot evidence bundle for QASH pilot-baseline-v0.2.
# Packages public evidence artifacts and verifies privacy boundaries before output.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEMO_DIR="${DEMO_DIR:-.qash-mvp-demo}"
OUT="${REPO_ROOT}/artifacts/pilot-baseline-v0.2"
DRY_RUN=0

for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
        *) echo "Unknown argument: $arg" >&2; exit 1 ;;
    esac
done

cd "$REPO_ROOT"

echo "=== QASH Pilot Evidence Bundle ==="
echo "Output: $OUT"
echo "Demo workspace: $DEMO_DIR"
[ "$DRY_RUN" = "1" ] && echo "(dry-run: no files will be written)"
echo

# ---- Require evidence files ----
require_file() {
    local path="$1" label="$2"
    if [ ! -f "$path" ]; then
        echo "ERROR: required file missing: $path ($label)" >&2
        echo "Hint: run 'bash scripts/run_mvp_demo.sh --clean' first." >&2
        exit 1
    fi
}

require_file "$DEMO_DIR/public_commitments.bin" "public commitments export"
require_file "$DEMO_DIR/replay.json"            "replay report"
require_file "$DEMO_DIR/disclosure.bin"         "selective disclosure bundle"
require_file "docs/mvp/claims_register.md"      "claims register"
require_file "docs/mvp/post_merge_audit.md"     "post-merge audit"

# ---- Privacy boundary checks ----
echo "--- Privacy boundary checks ---"

# public_commitments.bin: binary file — check for literal ASCII private-body strings.
# The WAL format does not base64-encode bodies, so a literal substring match suffices
# for the synthetic demo body used in run_mvp_demo.sh.
PRIV_STRINGS=("synthetic door alarm" "synthetic offline incident" "private body" "INCIDENT")
for s in "${PRIV_STRINGS[@]}"; do
    if grep -qF "$s" "$DEMO_DIR/public_commitments.bin" 2>/dev/null; then
        echo "ERROR: public_commitments.bin contains private body text: '$s'" >&2
        exit 1
    fi
done
echo "public_commitments.bin: no private body text found"

# replay.json: must have private_payloads_seen == false and no body text.
if python3 - <<'EOF'
import json, sys
with open(f"{os.environ.get('DEMO_DIR', '.qash-mvp-demo')}/replay.json") as f:
    d = json.load(f)
assert not d.get("private_payloads_seen", True), "private_payloads_seen is not false"
assert "profile_version" in d, "profile_version missing from replay report"
EOF
then
    echo "replay.json: privacy and schema checks passed"
else
    echo "ERROR: replay.json failed privacy or schema check" >&2
    exit 1
fi

for s in "${PRIV_STRINGS[@]}"; do
    if grep -qF "$s" "$DEMO_DIR/replay.json" 2>/dev/null; then
        echo "ERROR: replay.json contains private body text: '$s'" >&2
        exit 1
    fi
done
echo "replay.json: no private body text found"

# disclosure.bin: must contain exactly one receipt (size check — one disclosure record).
# The MVP disclosure format is checked structurally; here we only verify file is non-empty.
if [ ! -s "$DEMO_DIR/disclosure.bin" ]; then
    echo "ERROR: disclosure.bin is empty" >&2
    exit 1
fi
echo "disclosure.bin: non-empty"
echo

# ---- Copy files ----
if [ "$DRY_RUN" = "0" ]; then
    mkdir -p "$OUT"
    cp "$DEMO_DIR/public_commitments.bin" "$OUT/"
    cp "$DEMO_DIR/replay.json"            "$OUT/"
    cp "$DEMO_DIR/disclosure.bin"         "$OUT/"
    cp "docs/mvp/claims_register.md"     "$OUT/"
    cp "docs/mvp/post_merge_audit.md"    "$OUT/"

    # Optional docs (included if present)
    for opt in docs/mvp/operator_runbook.md docs/mvp/passive_observability.md; do
        [ -f "$opt" ] && cp "$opt" "$OUT/" && echo "included: $opt"
    done

    echo "--- Bundle contents ---"
    ls -lh "$OUT/"
    echo
    echo "Pilot evidence bundle written to: $OUT/"
else
    echo "(dry-run complete — bundle would be written to $OUT/)"
fi
