#!/usr/bin/env bash
# Build an amd64 .deb that installs chemlab-server, the web UI, and systemd unit.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v dpkg-deb >/dev/null 2>&1; then
    echo "dpkg-deb is required (install dpkg-dev on Ubuntu)" >&2
    exit 1
fi

ARCH="$(dpkg --print-architecture 2>/dev/null || echo unknown)"
if [ "$ARCH" != "amd64" ]; then
    echo "This script builds an amd64 package (host architecture: $ARCH)" >&2
    exit 1
fi

VERSION="$(
    awk '
        $0 == "[workspace.package]" { in_pkg = 1; next }
        in_pkg && $1 == "version" {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' Cargo.toml
)"
if [ -z "${VERSION:-}" ]; then
    echo "failed to read workspace.package version from Cargo.toml" >&2
    exit 1
fi

echo "==> cargo build --release -p chemlab-server"
cargo build --release -p chemlab-server

echo "==> npm ci && npm run build (apps/web)"
(
    cd apps/web
    npm ci
    npm run build
)

if [ ! -x target/release/chemlab-server ]; then
    echo "missing target/release/chemlab-server" >&2
    exit 1
fi
if [ ! -f apps/web/dist/index.html ]; then
    echo "missing apps/web/dist/index.html (web build failed?)" >&2
    exit 1
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

mkdir -p \
    "$STAGE/DEBIAN" \
    "$STAGE/usr/bin" \
    "$STAGE/usr/share/chemlab/www" \
    "$STAGE/etc/chemlab" \
    "$STAGE/lib/systemd/system" \
    "$STAGE/var/lib/chemlab"

install -m 0755 target/release/chemlab-server "$STAGE/usr/bin/chemlab-server"
cp -a apps/web/dist/. "$STAGE/usr/share/chemlab/www/"
# Not world-writable; postinst also forces root:chemlab 0640.
install -m 0640 packaging/deb/chemlab.env "$STAGE/etc/chemlab/chemlab.env"
install -m 0644 packaging/deb/chemlab.service "$STAGE/lib/systemd/system/chemlab.service"

SIZE_KB="$(du -sk "$STAGE" | awk '{print $1}')"
sed "s/@VERSION@/${VERSION}/g" packaging/deb/debian/control >"$STAGE/DEBIAN/control"
printf 'Installed-Size: %s\n' "$SIZE_KB" >>"$STAGE/DEBIAN/control"

install -m 0755 packaging/deb/debian/postinst "$STAGE/DEBIAN/postinst"
install -m 0755 packaging/deb/debian/prerm "$STAGE/DEBIAN/prerm"
install -m 0644 packaging/deb/debian/conffiles "$STAGE/DEBIAN/conffiles"

mkdir -p dist
DEB="dist/chemlab_${VERSION}_amd64.deb"
rm -f "$DEB"
dpkg-deb --root-owner-group --build "$STAGE" "$DEB"

echo "==> $DEB"
dpkg-deb --info "$DEB"
dpkg-deb --contents "$DEB"
