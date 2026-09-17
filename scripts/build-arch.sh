#!/usr/bin/env bash
# Build an x86_64 pacman package that installs chemlab-server, the web UI, and systemd unit.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [ "$(id -u)" -eq 0 ]; then
    if [ -z "${CI:-}" ] && [ -z "${GITHUB_ACTIONS:-}" ]; then
        echo "makepkg cannot run as root; rerun without sudo" >&2
        exit 1
    fi
    BUILDER="${CHEMLAB_BUILDER_USER:-builder}"
    if ! id "$BUILDER" >/dev/null 2>&1; then
        useradd --create-home --shell /bin/bash "$BUILDER"
    fi
    chown -R "$BUILDER":"$BUILDER" "$ROOT"
    exec runuser -u "$BUILDER" -- env CI="${CI:-}" GITHUB_ACTIONS="${GITHUB_ACTIONS:-}" "$0" "$@"
fi

if ! command -v makepkg >/dev/null 2>&1; then
    echo "makepkg is required (pacman -S --needed base-devel on Omarchy/Arch)" >&2
    exit 1
fi

ARCH="$(uname -m)"
if [ "$ARCH" != "x86_64" ]; then
    echo "This script builds an x86_64 package (host architecture: $ARCH)" >&2
    exit 1
fi

VERSION="$("$ROOT/scripts/build-release.sh" --print-version)"
"$ROOT/scripts/build-release.sh"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

# Substitute repo root with a path that is safe inside the generated PKGBUILD.
escaped_root="$(printf '%s' "$ROOT" | sed 's/[&|]/\\&/g')"
sed -e "s/@VERSION@/${VERSION}/g" -e "s|@ROOT@|${escaped_root}|g" \
    packaging/arch/PKGBUILD >"$STAGE/PKGBUILD"
install -m 0644 packaging/arch/chemlab.install "$STAGE/chemlab.install"

(
    cd "$STAGE"
    makepkg -f --nodeps
)

mkdir -p dist
PKG="dist/chemlab-${VERSION}-1-x86_64.pkg.tar.zst"
rm -f dist/chemlab-*.pkg.tar.zst
shopt -s nullglob
built=( "$STAGE"/chemlab-*.pkg.tar.zst )
if [ "${#built[@]}" -ne 1 ]; then
    echo "expected one .pkg.tar.zst from makepkg, found: ${built[*]:-none}" >&2
    exit 1
fi
mv "${built[0]}" "$PKG"

echo "==> $PKG"
tar -tf "$PKG"
if command -v pacman >/dev/null 2>&1; then
    pacman -Qlp "$PKG" || true
fi
if command -v namcap >/dev/null 2>&1; then
    namcap "$PKG" || true
fi
