#!/usr/bin/env bash
# Build a .deb that installs chemlab-server, the web UI, and systemd unit.
# Defaults: ARCH=amd64 (host build). For Raspberry Pi cross-build, see build-deb-pi.sh
# or set ARCH=arm64 TARGET=aarch64-unknown-linux-gnu.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v dpkg-deb >/dev/null 2>&1; then
    echo "dpkg-deb is required (install dpkg-dev on Ubuntu)" >&2
    exit 1
fi

ARCH="${ARCH:-amd64}"
# Optional cargo triple for cross-compile (empty = host / native).
TARGET="${TARGET:-}"

HOST_ARCH="$(dpkg --print-architecture 2>/dev/null || echo unknown)"
if [ -z "$TARGET" ] && [ "$ARCH" != "$HOST_ARCH" ]; then
    echo "ARCH=$ARCH but host is $HOST_ARCH; set TARGET=<cargo-triple> for cross-compile (see build-deb-pi.sh)" >&2
    exit 1
fi

VERSION="$("$ROOT/scripts/build-release.sh" --print-version)"
if [ -n "$TARGET" ]; then
    TARGET="$TARGET" "$ROOT/scripts/build-release.sh" --target "$TARGET"
    BINARY="target/${TARGET}/release/chemlab-server"
else
    "$ROOT/scripts/build-release.sh"
    BINARY="target/release/chemlab-server"
fi

if [ ! -x "$BINARY" ]; then
    echo "missing $BINARY" >&2
    exit 1
fi

STAGE="$(mktemp -d)"
chmod 0755 "$STAGE"
trap 'rm -rf "$STAGE"' EXIT

mkdir -p \
    "$STAGE/DEBIAN" \
    "$STAGE/usr/bin" \
    "$STAGE/usr/share/chemlab/www" \
    "$STAGE/etc/chemlab" \
    "$STAGE/lib/systemd/system"
# Match postinst: SQLite state dir must not be world-traversable (pacman warns if package is 755).
install -d -m 0750 "$STAGE/var/lib/chemlab"

install -m 0755 "$BINARY" "$STAGE/usr/bin/chemlab-server"
cp -a apps/web/dist/. "$STAGE/usr/share/chemlab/www/"
# Not world-writable; postinst also forces root:chemlab 0640.
install -m 0640 packaging/common/chemlab.env "$STAGE/etc/chemlab/chemlab.env"
install -m 0644 packaging/common/chemlab.service "$STAGE/lib/systemd/system/chemlab.service"

SIZE_KB="$(du -sk --exclude=DEBIAN "$STAGE" | awk '{print $1}')"
sed -e "s/@VERSION@/${VERSION}/g" -e "s/@ARCH@/${ARCH}/g" packaging/deb/debian/control |
    awk -v size="$SIZE_KB" '
        /^Description:/ { print "Installed-Size: " size }
        { print }
    ' >"$STAGE/DEBIAN/control"

install -m 0755 packaging/deb/debian/postinst "$STAGE/DEBIAN/postinst"
install -m 0755 packaging/deb/debian/prerm "$STAGE/DEBIAN/prerm"
install -m 0644 packaging/deb/debian/conffiles "$STAGE/DEBIAN/conffiles"

mkdir -p dist
DEB="dist/chemlab_${VERSION}_${ARCH}.deb"
rm -f "$DEB"
dpkg-deb --root-owner-group --build "$STAGE" "$DEB"

echo "==> $DEB"
dpkg-deb --info "$DEB"
dpkg-deb --contents "$DEB"
