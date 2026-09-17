#!/usr/bin/env bash
# Build release artifacts: chemlab-server binary + web UI dist.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

read_workspace_version() {
    awk '
        $0 == "[workspace.package]" { in_pkg = 1; next }
        in_pkg && $1 == "version" {
            gsub(/"/, "", $3)
            print $3
            exit
        }
    ' Cargo.toml
}

VERSION="$(read_workspace_version)"
if [ -z "${VERSION:-}" ]; then
    echo "failed to read workspace.package version from Cargo.toml" >&2
    exit 1
fi

if [ "${1:-}" = "--print-version" ]; then
    printf '%s\n' "$VERSION"
    exit 0
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
