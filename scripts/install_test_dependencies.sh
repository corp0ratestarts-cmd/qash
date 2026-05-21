#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

apt_packages=(
  build-essential
  ca-certificates
  curl
  pkg-config
  binutils-dev
  libunwind-dev
  liblzma-dev
  coq
  qemu-user-static
  gcc-aarch64-linux-gnu
  libc6-arm64-cross
  gcc-riscv64-linux-gnu
  libc6-riscv64-cross
)

if [[ -n "${APT_GET:-}" ]]; then
  read -r -a apt_get <<< "$APT_GET"
elif [[ "$(id -u)" -eq 0 ]]; then
  apt_get=(apt-get)
elif command -v sudo >/dev/null 2>&1; then
  apt_get=(sudo -n apt-get)
else
  echo "error: apt package installation requires root or sudo" >&2
  exit 1
fi

if [[ "${SKIP_APT:-0}" == "1" ]]; then
  echo "==> Skipping system packages because SKIP_APT=1"
else
  echo "==> Installing system packages"
  "${apt_get[@]}" update -qq
  "${apt_get[@]}" install -y --no-install-recommends "${apt_packages[@]}"
fi

echo "==> Installing pinned Rust toolchain"
rustup toolchain install
scripts/verify_rust_toolchain.sh

if ! rustup target list --installed | grep -qx 'aarch64-unknown-linux-gnu'; then
  echo "==> Installing Rust target aarch64-unknown-linux-gnu"
  rustup target add aarch64-unknown-linux-gnu
fi

if ! rustup target list --installed | grep -qx 'riscv64gc-unknown-linux-gnu'; then
  echo "==> Installing Rust target riscv64gc-unknown-linux-gnu"
  rustup target add riscv64gc-unknown-linux-gnu
fi

if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "==> Installing cargo-deny"
  cargo install cargo-deny --locked
else
  echo "==> cargo-deny already installed"
fi

if ! command -v cargo-hfuzz >/dev/null 2>&1; then
  echo "==> Installing cargo-honggfuzz"
  cargo install honggfuzz
else
  echo "==> cargo-honggfuzz already installed"
fi

if ! cargo kani --version 2>/dev/null | grep -q '0\.67\.0'; then
  echo "==> Installing kani-verifier 0.67.0"
  cargo install kani-verifier --version 0.67.0 --locked
else
  echo "==> kani-verifier 0.67.0 already installed"
fi

echo "==> Setting up Kani"
cargo kani setup

echo "==> Local test dependency installation complete"
