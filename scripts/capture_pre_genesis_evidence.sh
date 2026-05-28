#!/usr/bin/env bash
# Capture the minimum pre-genesis evidence bundle for the current worktree.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

SLICE_DOC="docs/release/current_integration_review_slices.md"
declare -A COMMAND_STATUS
declare -A COMMAND_LOG

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
commit="$(git rev-parse --short=12 HEAD)"
out_dir="${1:-artifacts/evidence/${timestamp}-${commit}}"
log_dir="$out_dir/logs"
manifest="$out_dir/manifest.txt"

mkdir -p "$log_dir"

tmp_cargo_home=""
cleanup() {
    if [ -n "$tmp_cargo_home" ] && [ -d "$tmp_cargo_home" ]; then
        rm -rf "$tmp_cargo_home"
    fi
}
trap cleanup EXIT

record_command_status() {
    local command="$1"
    local status="$2"
    local log_path="$3"
    COMMAND_STATUS["$command"]="$status"
    COMMAND_LOG["$command"]="$log_path"
}

run_step() {
    local name="$1"
    shift

    local log="$log_dir/${name}.log"
    printf '== %s ==\n' "$name" | tee -a "$manifest"
    printf '$ %s\n' "$*" | tee -a "$manifest"

    set +e
    "$@" >"$log" 2>&1
    local status=$?
    set -e

    if [ "$status" -eq 0 ]; then
        printf 'status: PASS\n\n' | tee -a "$manifest"
        record_command_status "$*" "PASS" "$log"
    else
        printf 'status: FAIL (%s)\nlog: %s\n\n' "$status" "$log" | tee -a "$manifest"
        record_command_status "$*" "FAIL ($status)" "$log"
        return "$status"
    fi
}

capture_cargo_metadata() {
    local log="$log_dir/cargo_metadata.log"
    local metadata="$out_dir/cargo-metadata.json"

    printf '== cargo_metadata ==\n' | tee -a "$manifest"
    printf '$ cargo metadata --offline --format-version 1 > %s\n' "$metadata" | tee -a "$manifest"

    set +e
    cargo metadata --offline --format-version 1 >"$metadata" 2>"$log"
    local status=$?
    set -e

    if [ "$status" -eq 0 ]; then
        printf 'status: PASS\n\n' | tee -a "$manifest"
        record_command_status "cargo metadata --offline --format-version 1" "PASS" "$log"
    else
        printf 'status: FAIL (%s)\nlog: %s\n\n' "$status" "$log" | tee -a "$manifest"
        record_command_status "cargo metadata --offline --format-version 1" "FAIL ($status)" "$log"
        return "$status"
    fi
}

emit_oscal_assessment() {
    # Emit an OSCAL-style Assessment Results stub (NIST OSCAL 1.1, simplified).
    # Records evidence bundle metadata and pass/fail per control activity.
    # Full OSCAL validation requires oscal-cli; this stub is machine-readable
    # and suitable for submission to a compliance automation pipeline.
    local oscal_file="$out_dir/oscal-assessment.json"

    local pass_count=0
    local fail_count=0
    for cmd in "${!COMMAND_STATUS[@]}"; do
        if [ "${COMMAND_STATUS[$cmd]}" = "PASS" ]; then
            pass_count=$((pass_count + 1))
        else
            fail_count=$((fail_count + 1))
        fi
    done
    local overall="pass"
    [ "$fail_count" -gt 0 ] && overall="fail"

    # Build tab-separated cmd\tstatus lines for Python to consume as stdin.
    {
        for cmd in "${!COMMAND_STATUS[@]}"; do
            printf '%s\t%s\n' "$cmd" "${COMMAND_STATUS[$cmd]}"
        done
    } | python3 - "$timestamp" "$commit" "$overall" "$pass_count" "$fail_count" > "$oscal_file" <<'PY'
import json, sys

ts, commit, overall, pass_count, fail_count = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

results = []
for line in sys.stdin.read().strip().splitlines():
    if '\t' not in line:
        continue
    cmd, status = line.split('\t', 1)
    results.append({"title": cmd.strip(), "result": "pass" if status.strip() == "PASS" else "fail"})

doc = {
    "component-definition": {
        "uuid": "00000000-0000-0000-0000-000000000001",
        "metadata": {
            "title": "QASH Pre-Genesis Evidence Assessment",
            "last-modified": ts, "version": commit, "oscal-version": "1.1.0",
        },
        "components": [{"uuid": "00000000-0000-0000-0000-000000000002", "type": "software",
            "title": "QASH Consensus Engine",
            "description": "Post-quantum deterministic consensus kernel and PAL.",
            "control-implementations": [{"uuid": "00000000-0000-0000-0000-000000000003",
                "source": "https://pages.nist.gov/OSCAL/",
                "description": "Pre-genesis evidence gate activities",
                "implemented-requirements": [
                    {"uuid": "00000000-0000-{:04d}-0000-000000000000".format(i),
                     "control-id": "evidence-{:03d}".format(i+1),
                     "description": r["title"], "remarks": r["result"]}
                    for i, r in enumerate(results)
                ]}]}]},
    "assessment-results": {
        "uuid": "00000000-0000-0000-0000-000000000099",
        "metadata": {"title": "QASH Evidence Assessment Results",
            "last-modified": ts, "version": commit, "oscal-version": "1.1.0"},
        "import-ap": {"href": "#"},
        "results": [{"uuid": "00000000-0000-0000-0000-000000000100",
            "title": "Evidence bundle run", "start": ts, "end": ts,
            "reviewed-controls": {"control-selections": [{"include-all": {}}]},
            "findings": [
                {"uuid": "00000000-0001-{:04d}-0000-000000000000".format(i+1),
                 "title": r["title"],
                 "target": {"type": "objective-id",
                     "target-id": "evidence-{:03d}".format(i+1),
                     "status": {"state": r["result"]}}}
                for i, r in enumerate(results)
            ],
            "attestations": [{"responsible-parties": [{"role-id": "assessor",
                "party-uuids": ["00000000-0000-0000-0000-000000000200"]}],
                "parts": [{"name": "assessment-log-entry",
                    "prose": "Automated evidence bundle. Overall: {}. Pass: {}. Fail: {}.".format(
                        overall, pass_count, fail_count)}]}]}]}
}
print(json.dumps(doc, indent=2))
PY

    printf '== oscal_assessment ==\n' | tee -a "$manifest"
    printf 'oscal_file: %s\n' "$oscal_file" | tee -a "$manifest"
    printf 'overall: %s (pass=%s fail=%s)\n\n' "$overall" "$pass_count" "$fail_count" | tee -a "$manifest"
}

append_slice_command_statuses() {
    printf '## Slice Command Statuses\n\n' >>"$manifest"
    printf '| Slice | Command | Status | Log |\n' >>"$manifest"
    printf '| --- | --- | --- | --- |\n' >>"$manifest"

    if [ ! -f "$SLICE_DOC" ]; then
        printf '| Slice 0 | `%s` | PASS | self |\n\n' "bash scripts/capture_pre_genesis_evidence.sh" >>"$manifest"
        return
    fi

    python3 - "$SLICE_DOC" <<'PY' | while IFS=$'\t' read -r slice command; do
import re
import sys
from pathlib import Path

text = Path(sys.argv[1]).read_text(encoding="utf-8")
current_slice = None
in_required = False
for line in text.splitlines():
    m = re.match(r"## Slice (\d+):", line)
    if m:
        current_slice = f"Slice {m.group(1)}"
        in_required = False
        continue
    if current_slice is None:
        continue
    if line.startswith("Required evidence:"):
        in_required = True
        continue
    if in_required and (line.startswith("Review focus:") or line.startswith("## ")):
        in_required = False
        continue
    if in_required:
        cmd = re.match(r"- `(.+)`", line.strip())
        if cmd:
            print(f"{current_slice}\t{cmd.group(1)}")
PY
        status="${COMMAND_STATUS[$command]:-NOT RUN}"
        log_path="${COMMAND_LOG[$command]:--}"
        if [ "$command" = "bash scripts/capture_pre_genesis_evidence.sh" ]; then
            status="PASS"
            log_path="self"
        fi
        printf '| %s | `%s` | %s | %s |\n' "$slice" "$command" "$status" "$log_path" >>"$manifest"
    done
    printf '\n' >>"$manifest"
}

prepare_cargo_deny_home() {
    tmp_cargo_home="$(mktemp -d /tmp/qash-cargo-deny.XXXXXX)"

    local source_home="${CARGO_HOME:-$HOME/.cargo}"
    if [ -d "$source_home/advisory-dbs" ]; then
        mkdir -p "$tmp_cargo_home"
        cp -a "$source_home/advisory-dbs" "$tmp_cargo_home/"
    fi

    printf '%s' "$tmp_cargo_home"
}

{
    printf '# Pre-Genesis Evidence Bundle\n\n'
    printf 'Captured (UTC): %s\n' "$timestamp"
    printf 'Evidence freshness timestamp (UTC): %s\n' "$timestamp"
    printf 'Commit: %s\n' "$(git rev-parse HEAD)"
    printf 'Commit short: %s\n' "$commit"
    printf 'Rust: %s\n' "$(rustc --version 2>/dev/null || printf 'not found')"
    printf 'Cargo: %s\n' "$(cargo --version 2>/dev/null || printf 'not found')"
    printf 'Coq: %s\n' "$(coqc --version 2>/dev/null | head -1 || printf 'not found')"
    printf 'Kani: %s\n' "$(cargo kani --version 2>/dev/null || kani --version 2>/dev/null || printf 'not found')"
    printf '\n## Working Tree\n\n'
    status_excludes=()
    case "$out_dir" in
        /*) ;;
        *) status_excludes=(":(exclude)$out_dir" ":(exclude)$out_dir/**") ;;
    esac
    git status --short -- . "${status_excludes[@]}"
    printf '\n## Command Summary\n\n'
} >"$manifest"

run_step document_hygiene bash scripts/check_document_hygiene.sh
run_step privacy_admission bash scripts/check_privacy_admission.sh
run_step diff_check git diff --check
run_step fmt_check cargo fmt --all -- --check
run_step v1_2_sharded_replay cargo test -p qash-consensus --test v1_2_sharded_replay
run_step vector_integrity cargo test -p qash-consensus --test vector_integrity
run_step coq_refinement_vectors cargo test -p qash-consensus --test coq_refinement_vectors
run_step phase2r_preconditions cargo test -p qash-consensus --test phase2r_preconditions
run_step consensus_bench_compile cargo bench -p qash-consensus --no-run
run_step workspace_tests cargo test --workspace
run_step pal_std_tests cargo test -p qash-pal --features std
run_step proofs make -C proofs
cargo_metadata_path="$out_dir/cargo-metadata.json"
capture_cargo_metadata
cargo_deny_home="$(prepare_cargo_deny_home)"
run_step supply_chain env CARGO_HOME="$cargo_deny_home" cargo deny check --disable-fetch --metadata-path "$cargo_metadata_path"
run_step kani_consensus scripts/run_kani_consensus.sh
append_slice_command_statuses
emit_oscal_assessment

printf 'Evidence bundle written to %s\n' "$out_dir"
