#!/usr/bin/env bash
# Capture the minimum pre-genesis evidence bundle for the current worktree.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

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
    else
        printf 'status: FAIL (%s)\nlog: %s\n\n' "$status" "$log" | tee -a "$manifest"
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
    else
        printf 'status: FAIL (%s)\nlog: %s\n\n' "$status" "$log" | tee -a "$manifest"
        return "$status"
    fi
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
    printf 'Captured: %s\n' "$timestamp"
    printf 'Commit: %s\n' "$(git rev-parse HEAD)"
    printf 'Commit short: %s\n' "$commit"
    printf 'Rust: %s\n' "$(rustc --version 2>/dev/null || printf 'not found')"
    printf 'Cargo: %s\n' "$(cargo --version 2>/dev/null || printf 'not found')"
    printf 'Coq: %s\n' "$(coqc --version 2>/dev/null | head -1 || printf 'not found')"
    printf 'Kani: %s\n' "$(cargo kani --version 2>/dev/null || kani --version 2>/dev/null || printf 'not found')"
    printf '\n## Working Tree\n\n'
    git status --short
    printf '\n## Command Summary\n\n'
} >"$manifest"

run_step document_hygiene bash scripts/check_document_hygiene.sh
run_step diff_check git diff --check
run_step fmt_check cargo fmt --all -- --check
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

printf 'Evidence bundle written to %s\n' "$out_dir"
