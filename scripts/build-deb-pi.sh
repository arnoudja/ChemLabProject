#!/usr/bin/env bash
# Cross-build an arm64 .deb for Raspberry Pi OS 64-bit (aarch64).
#
# Prerequisites (on an amd64 Ubuntu/Debian build host):
#   - Rust toolchain (see rust-toolchain.toml) + rustup target aarch64-unknown-linux-gnu
#   - Cross linker: sudo apt install gcc-aarch64-linux-gnu
#   - dpkg-deb (dpkg-dev), Node 22+ for the web UI
#
# The linker is picked up via CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER when unset.
# ChemLab uses rustls + sqlx sqlite (bundled); no OpenSSL pkg-config cross setup is required.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

export ARCH=arm64
export TARGET=aarch64-unknown-linux-gnu

if ! command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
    echo "aarch64-linux-gnu-gcc not found." >&2
    echo "Install the cross linker: sudo apt install gcc-aarch64-linux-gnu" >&2
    exit 1
fi

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="${CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER:-aarch64-linux-gnu-gcc}"

echo "==> Raspberry Pi arm64 deb (ARCH=$ARCH TARGET=$TARGET)"
"$ROOT/scripts/build-deb.sh"

VERSION="$("$ROOT/scripts/build-release.sh" --print-version)"
DEB="dist/chemlab_${VERSION}_${ARCH}.deb"
echo
echo "==> Install on Raspberry Pi OS 64-bit:"
echo "    sudo dpkg -i $DEB"
echo "    # or: sudo apt install ./$DEB"
