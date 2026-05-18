#!/usr/bin/env bash
# apply_patches.sh
# Applies all QASH no_std/entry-point fixes, commits, and pushes.
#
# ONE-TIME SETUP (run these manually before this script, once ever):
#
#   sudo apt install gh git
#   gh auth login --git-protocol https --web
#   gh auth setup-git
#
# After that this script can be run any time without re-logging in.
#
# Usage (from inside your cloned qash repo):
#   bash apply_patches.sh

set -euo pipefail

# ---------------------------------------------------------------------------
# 0. Pre-flight: check gh auth and set persistent git credential helper
# ---------------------------------------------------------------------------

echo "==> Checking gh authentication..."
if ! gh auth status > /dev/null 2>&1; then
  echo ""
  echo "ERROR: Not logged in to GitHub CLI."
  echo "Run this once to fix it permanently:"
  echo ""
  echo "  gh auth login --git-protocol https --web"
  echo "  gh auth setup-git"
  echo ""
  exit 1
fi
echo "  [OK] gh authenticated"

gh auth setup-git
echo "  [OK] git credential helper set to gh"

REPO_ROOT="$(git rev-parse --show-toplevel)"
echo "==> Repo root: $REPO_ROOT"
cd "$REPO_ROOT"

# ---------------------------------------------------------------------------
# 1. src/main.rs — remove no_std, clean hosted entrypoint
# ---------------------------------------------------------------------------
cat > src/main.rs << 'PATCH'
// src/main.rs
// Hosted binary entrypoint. std is available here.
// The no_std invariant lives in crates/consensus, not here.
#![forbid(unsafe_code)]

fn main() {
    let data = b"genesis";
    let hash = qash_consensus::consensus_hash(data);
    println!("qash consensus hash: {:02x?}", hash);
}
PATCH
echo "  [OK] src/main.rs"

# ---------------------------------------------------------------------------
# 2. crates/pal/Cargo.toml — add std feature definition
# ---------------------------------------------------------------------------
cat > crates/pal/Cargo.toml << 'PATCH'
[package]
name = "qash-pal"
version = "0.1.0"
edition = "2021"

[dependencies]
qash-consensus = { path = "../consensus" }

[features]
default = []
std = []
PATCH
echo "  [OK] crates/pal/Cargo.toml"

# ---------------------------------------------------------------------------
# 3. crates/pal/src/lib.rs — proper std feature gating
# ---------------------------------------------------------------------------
cat > crates/pal/src/lib.rs << 'PATCH'
// Platform Abstraction Layer (PAL)
// Traits are the only interface the consensus core ever sees.
// Implementations are feature-gated so the core stays pure no_std.

pub trait Time   { fn epoch_counter() -> u64; }
pub trait Net    { fn send(data: &[u8]); fn recv(buf: &mut [u8]) -> usize; }
pub trait Attest { fn tpm_quote() -> [u8; 256]; }
pub trait Halt   { fn absorbing_reset() -> !; }

#[cfg(feature = "std")]
pub mod hosted {
    use super::*;

    pub struct Host;

    impl Time for Host {
        fn epoch_counter() -> u64 { 0 }
    }

    impl Net for Host {
        fn send(_data: &[u8]) {}
        fn recv(_buf: &mut [u8]) -> usize { 0 }
    }

    impl Attest for Host {
        fn tpm_quote() -> [u8; 256] { [0u8; 256] }
    }

    impl Halt for Host {
        fn absorbing_reset() -> ! { std::process::exit(1) }
    }
}
PATCH
echo "  [OK] crates/pal/src/lib.rs"

# ---------------------------------------------------------------------------
# 4. .github/workflows/ci.yml — reproducibility flags + LLD
# ---------------------------------------------------------------------------
mkdir -p .github/workflows
cat > .github/workflows/ci.yml << 'PATCH'
name: QASH CI
on: [push, pull_request]

env:
  SOURCE_DATE_EPOCH: "0"
  CARGO_INCREMENTAL: "0"
  RUSTFLAGS: "-C debuginfo=0 -C link-arg=--build-id=none"

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: stable

      - name: Install LLD
        run: sudo apt-get install -y lld

      - name: Build & Test (consensus core)
        run: cargo test --no-default-features

      - name: Build PAL (std path)
        run: cargo build -p qash-pal --features std

      - name: Verify no unsafe in consensus core
        run: |
          if grep -r "unsafe" crates/consensus/src; then
            echo "ERROR: unsafe code found in consensus core"
            exit 1
          fi
          echo "OK: no unsafe in consensus core"
PATCH
echo "  [OK] .github/workflows/ci.yml"

# ---------------------------------------------------------------------------
# 5. rust-toolchain.toml — pinned toolchain
# ---------------------------------------------------------------------------
cat > rust-toolchain.toml << 'PATCH'
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
targets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "riscv64gc-unknown-linux-gnu",
]
PATCH
echo "  [OK] rust-toolchain.toml"

# ---------------------------------------------------------------------------
# 6. Verify the build compiles before committing
# ---------------------------------------------------------------------------
echo ""
echo "==> Running cargo check to verify patches compile..."
cargo check --no-default-features
cargo check -p qash-pal --features std
echo "  [OK] cargo check passed"

# ---------------------------------------------------------------------------
# 7. Stage, commit, push
# ---------------------------------------------------------------------------
echo ""
echo "==> Committing and pushing..."
git add \
  src/main.rs \
  crates/pal/Cargo.toml \
  crates/pal/src/lib.rs \
  .github/workflows/ci.yml \
  rust-toolchain.toml

git commit -m "fix: resolve no_std/main inconsistency and harden CI reproducibility

- src/main.rs: remove #![no_std]; hosted binary uses std entrypoint
- crates/pal: add std feature gate; Host impl behind #[cfg(feature = \"std\")]
- ci.yml: add SOURCE_DATE_EPOCH=0, CARGO_INCREMENTAL=0, --build-id=none
- rust-toolchain.toml: pin stable toolchain with cross-compile targets"

git push

echo ""
echo "==> All done."
echo "    Monitor CI:      gh run watch"
echo "    Or view browser: gh repo view --web"
