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

VERSION="$("$ROOT/scripts/build-release.sh" --print-version)"
"$ROOT/scripts/build-release.sh"

STAGE="$(mktemp -d)"
chmod 0755 "$STAGE"
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
install -m 0640 packaging/common/chemlab.env "$STAGE/etc/chemlab/chemlab.env"
install -m 0644 packaging/common/chemlab.service "$STAGE/lib/systemd/system/chemlab.service"

SIZE_KB="$(du -sk --exclude=DEBIAN "$STAGE" | awk '{print $1}')"
sed "s/@VERSION@/${VERSION}/g" packaging/deb/debian/control |
    awk -v size="$SIZE_KB" '
        /^Description:/ { print "Installed-Size: " size }
        { print }
    ' >"$STAGE/DEBIAN/control"

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
