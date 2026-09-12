#!/usr/bin/env bash
# Regenerate TypeScript types from chemlab-contracts (ts-rs).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p apps/web/src/generated
cargo test -p chemlab-contracts --lib
cat > apps/web/src/generated/contracts.ts <<'EOF'
// Barrel for shared API contracts.
// Prefer the ts-rs generated modules; re-export for a stable import path.
export type { HealthResponse } from './HealthResponse'
export type { RegisterRequest } from './RegisterRequest'
export type { LoginRequest } from './LoginRequest'
export type { AuthUserResponse } from './AuthUserResponse'
export type { ErrorResponse } from './ErrorResponse'
export type { MeResponse } from './MeResponse'
export type { CsrfResponse } from './CsrfResponse'
export type { DissolveRequest } from './DissolveRequest'
export type { DissolveResponse } from './DissolveResponse'
EOF
echo "OK: apps/web/src/generated/"
ls -1 apps/web/src/generated/
