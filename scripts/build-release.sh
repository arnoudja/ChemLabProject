#!/usr/bin/env bash
# Build release artifacts: chemlab-server binary + web UI dist.
# Optional: --target <triple> (or TARGET env) for cross-compile; default is host.
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

# Packagers may export TARGET=<triple>; --target wins when both are set.
CARGO_TARGET="${TARGET:-}"
while [ $# -gt 0 ]; do
    case "$1" in
        --target)
            if [ -z "${2:-}" ]; then
                echo "usage: $0 [--target <triple>]" >&2
                exit 1
            fi
            CARGO_TARGET="$2"
            shift 2
            ;;
        --print-version)
            printf '%s\n' "$VERSION"
            exit 0
            ;;
        *)
            echo "usage: $0 [--target <triple>]" >&2
            exit 1
            ;;
    esac
done

CARGO_TARGET_ARGS=()
BINARY="target/release/chemlab-server"
if [ -n "${CARGO_TARGET}" ]; then
    if ! rustup target list --installed | grep -qx "$CARGO_TARGET"; then
        echo "==> rustup target add $CARGO_TARGET"
        rustup target add "$CARGO_TARGET"
    fi
    CARGO_TARGET_ARGS=(--target "$CARGO_TARGET")
    BINARY="target/${CARGO_TARGET}/release/chemlab-server"
fi

echo "==> cargo build --release -p chemlab-server${CARGO_TARGET:+ --target $CARGO_TARGET}"
cargo build --release -p chemlab-server "${CARGO_TARGET_ARGS[@]}"

echo "==> npm ci && npm run build (apps/web)"
(
    cd apps/web
    npm ci
    npm run build
)

if [ ! -x "$BINARY" ]; then
    echo "missing $BINARY" >&2
    exit 1
fi
if [ ! -f apps/web/dist/index.html ]; then
    echo "missing apps/web/dist/index.html (web build failed?)" >&2
    exit 1
fi

echo "==> binary: $BINARY"
export CHEMLAB_RELEASE_BINARY="$BINARY"
