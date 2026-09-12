#!/usr/bin/env bash

set -euo pipefail

echo "### Building the package ###"
rm -f ./dist/chemlab_*_amd64.deb
./scripts/build-deb.sh

echo "### Installing the package ###"
sudo apt-get install --reinstall ./dist/chemlab_*_amd64.deb

echo "### Done ###"
