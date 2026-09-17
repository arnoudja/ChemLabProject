#!/usr/bin/env bash

set -euo pipefail

if [ ! -f /etc/os-release ]; then
    echo "Cannot detect distro: /etc/os-release is missing" >&2
    exit 1
fi
# shellcheck disable=SC1091
. /etc/os-release

is_debian_like() {
    [ "$ID" = debian ] || [ "$ID" = ubuntu ] || [[ "${ID_LIKE:-}" == *debian* ]]
}

is_arch_like() {
    [ "$ID" = arch ] || [ "$ID" = omarchy ] || [[ "${ID_LIKE:-}" == *arch* ]]
}

echo "### Building the package ###"
if is_arch_like && ! is_debian_like; then
    rm -f ./dist/chemlab-*.pkg.tar.zst
    ./scripts/build-arch.sh
    echo "### Installing the package ###"
    sudo pacman -U ./dist/chemlab-*.pkg.tar.zst
elif is_debian_like; then
    rm -f ./dist/chemlab_*_amd64.deb
    ./scripts/build-deb.sh
    echo "### Installing the package ###"
    sudo apt-get install --reinstall ./dist/chemlab_*_amd64.deb
else
    echo "Unsupported distro: ${ID} (need Ubuntu/Debian or Omarchy/Arch)" >&2
    exit 1
fi

echo "### Done ###"
