# ChemLabProject

ChemLab is a web lab game that is fun and as realistic as practical.  
**v0.1** is framework plus the first chemistry API — welcome page, login/session stub, session-gated `POST /api/lab/dissolve` (NaCl vs sand), a signed-in picker, and a 2D spoon bench that both show the server result. No evaporate yet.

Primary browsers: **Firefox**. Backend targets **Linux** (Ubuntu / Omarchy). Production shape later: Raspberry Pi 4 + Caddy.

## Layout

```
Cargo.toml                 # workspace
crates/
  chemlab-core/            # domain (dissolve lookup: NaCl vs sand)
  chemlab-db/              # sqlx + SQLite migrations (users/sessions)
  chemlab-server/          # Axum HTTP API + SPA/static / Vite proxy
  chemlab-contracts/       # shared DTOs (serde + ts-rs)
apps/
  web/                     # Vite + React + TypeScript + Tailwind
scripts/
  generate-types.sh        # regenerate FE types from contracts
  build-deb.sh             # Ubuntu amd64 .deb (binary + web UI + systemd)
packaging/deb/             # systemd unit, env file, maintainer scripts
```

## Prerequisites (Ubuntu / Omarchy)

```bash
# Rust (pinned in rust-toolchain.toml)
curl https://sh.rustup.rs -sSf | sh
rustup show

# Node 22+ (for the Vite app)
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev sqlite3
# Install Node via nvm, nodesource, or your distro package — need npm.
node -v   # expect v22+
```

## Run locally (two processes)

Default ports avoid clashes with other apps:

| Process | Bind |
| --- | --- |
| Axum API | `127.0.0.1:3847` (or `0.0.0.0:3847` via `CHEMLAB_BIND`) |
| Vite FE  | `127.0.0.1:5179` (or `0.0.0.0:5179` via `CHEMLAB_VITE_HOST`) |

### 1. Backend

```bash
cd /path/to/ChemLabProject
cp -n .env.example .env   # optional; auto-loaded on startup
cargo run -p chemlab-server
```

Useful env vars (also in `.env.example`):

- `CHEMLAB_BIND` — default `127.0.0.1:3847` (use `0.0.0.0:3847` for LAN access)
- `CHEMLAB_DATABASE_URL` — default `sqlite://chemlab.db`
- `CHEMLAB_VITE_PROXY` — e.g. `http://127.0.0.1:5179` so Axum serves `/` by proxying Vite
- `CHEMLAB_STATIC_DIR` — e.g. `apps/web/dist` after `npm run build`
- `CHEMLAB_COOKIE_SECURE` — `true` behind HTTPS

Health check: [http://127.0.0.1:3847/api/health](http://127.0.0.1:3847/api/health)

### 2. Frontend

```bash
cd apps/web
npm install
npm run dev
```

Open [http://127.0.0.1:5179/](http://127.0.0.1:5179/). Vite proxies `/api/*` to Axum on `:3847`.

**LAN:** set `CHEMLAB_VITE_HOST=0.0.0.0` in the repo-root `.env`, restart `npm run dev`, then open
`http://<lan-ip>:5179/` from another machine (API proxy still targets local Axum on `:3847`).

**Alternate (single origin via Axum):** start Vite, then:

```bash
CHEMLAB_VITE_PROXY=http://127.0.0.1:5179 cargo run -p chemlab-server
```

Browse [http://127.0.0.1:3847/](http://127.0.0.1:3847/) — Axum forwards non-API paths to Vite.

### Production-like static serve

```bash
cd apps/web && npm run build
cd ../..
CHEMLAB_STATIC_DIR=apps/web/dist cargo run -p chemlab-server
```

## Install on Ubuntu (.deb)

The package installs `chemlab-server`, the built web UI, and a systemd unit that
starts on boot and listens on **`0.0.0.0:3847`**. `cargo run` defaults stay
`127.0.0.1:3847`.

Build on Ubuntu amd64 (needs Rust, Node 22+, `dpkg-deb`):

```bash
./scripts/build-deb.sh
sudo apt install ./dist/chemlab_*.deb
```

Then open `http://<host>:3847/`. Health: `http://<host>:3847/api/health`.

| Path | Role |
| --- | --- |
| `/usr/bin/chemlab-server` | release binary |
| `/usr/share/chemlab/www/` | production web UI (`apps/web` `dist/`) |
| `/etc/chemlab/chemlab.env` | daemon env (not world-writable) |
| `/lib/systemd/system/chemlab.service` | systemd unit (`User=chemlab`) |
| `/var/lib/chemlab/` | SQLite data dir |

The unit env is `CHEMLAB_BIND=0.0.0.0:3847`, `CHEMLAB_STATIC_DIR=/usr/share/chemlab/www`,
`CHEMLAB_DATABASE_URL=sqlite:///var/lib/chemlab/chemlab.db`, and
`CHEMLAB_COOKIE_SECURE=false` (HTTP on the LAN). `postinst` enables and starts
the service; `prerm` stops and disables it on remove.

```bash
sudo systemctl status chemlab
sudo journalctl -u chemlab -e
```

CI uploads the `.deb` as the `chemlab-deb` artifact. Caddy / HTTPS / loopback
bind are later changes.

## Auth stub (accounts from day one)

Working thin stub (not a fake button):

| Method | Path | Notes |
| --- | --- | --- |
| `GET`  | `/api/auth/csrf` | issues HttpOnly `chemlab_csrf` cookie + `{ csrf_token }` |
| `POST` | `/api/auth/register` | email, password (≥8), display_name → sets session cookie; requires CSRF; rate-limited |
| `POST` | `/api/auth/login` | email, password → rotates session (invalidates previous, sets new cookie); requires CSRF; rate-limited |
| `POST` | `/api/auth/logout` | clears cookie + deletes server session; requires CSRF |
| `GET`  | `/api/auth/me` | `{ authenticated, user }` |
| `POST` | `/api/lab/dissolve` | `{ substance_id, solvent_id, temperature_c }` → `{ dissolved, explanation }`; session + CSRF |

- Passwords: Argon2  
- Session cookie: `chemlab_session` (HttpOnly, SameSite=Lax; Secure when configured). Successful login issues a new session and invalidates the previous one.  
- CSRF: double-submit synchronizer — `GET /api/auth/csrf`, then send `X-CSRF-Token` matching `chemlab_csrf` on mutating POSTs (`/api/auth/*` and `/api/lab/dissolve` via `require_csrf`)  
- Rate limits: in-process sliding window on `POST /api/auth/register` and `POST /api/auth/login` (5 / 60s per IP and per email). Over limit → `429` `{ code: "rate_limited" }`. Logout is not limited.  
- Store: SQLite `users` + `sessions` (token **hash** only)

## Tests

```bash
# All Rust crates (health + auth + dissolve HTTP, db, core, contracts)
cargo test --workspace

# Frontend unit smoke (Vitest) — optional once installed
cd apps/web && npm test
```

CI runs `cargo test --workspace`, `cargo fmt --check`, `cargo clippy`, a frontend
build, and an amd64 `.deb` package job.

## Generate shared TS types

```bash
./scripts/generate-types.sh
```

v0.1 also keeps `apps/web/src/generated/contracts.ts` in sync by hand when needed; CI will tighten the dirty-check in a later milestone.

## Design notes

- Chemistry rules live in `chemlab-core` (server-authoritative). Browser never decides “did NaCl dissolve?”
- Login is required even for single-player (see project context).
- Welcome-page dissolve control and the 2D spoon bench show the server result only (issue #9); no evaporate.

## License

MIT — see [LICENSE](LICENSE).
