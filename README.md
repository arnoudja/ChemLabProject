# ChemLabProject

ChemLab is a web lab game that is fun and as realistic as practical.  
**v0.1** is framework plus the first chemistry API — welcome page, login/session stub, session-gated lab scene (`GET /api/lab/scene`, `POST /api/lab/action`) with a 2D spoon bench, plus predict-only `POST /api/lab/dissolve` (NaCl vs sand). No evaporate yet.

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
  build-release.sh         # cargo --release + Vite dist (used by packagers)
  build-deb.sh             # Ubuntu amd64 .deb (binary + web UI + systemd)
  build-arch.sh            # Omarchy/Arch x86_64 .pkg.tar.zst
packaging/common/          # systemd unit + env (shared)
packaging/deb/             # Debian control + maintainer scripts
packaging/arch/            # PKGBUILD + install script
packaging/caddy/           # Caddyfile (local/LAN HTTPS) + Caddyfile.internet-facing
```

## Prerequisites (Ubuntu / Omarchy)

Rust is pinned in `rust-toolchain.toml`. Node 22+ is required for the Vite app (`node -v`).

**Ubuntu / Debian**

```bash
curl https://sh.rustup.rs -sSf | sh
rustup show
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev sqlite3
# Install Node via nvm, nodesource, or your distro package — need npm.
```

**Omarchy / Arch**

```bash
curl https://sh.rustup.rs -sSf | sh
rustup show
sudo pacman -S --needed base-devel
# Node 22+ via mise (Omarchy default) or: sudo pacman -S --needed nodejs npm
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
- `CHEMLAB_COOKIE_SECURE` — `true` behind HTTPS (sets `Secure` on session, CSRF, and clear-session cookies)
- `CHEMLAB_SIGNUP_ENABLED` — `true` / `1` (case-insensitive) enables `POST /api/auth/register`; unset defaults to enabled. `false` returns 403 `{ code: "signup_disabled" }`
- `CHEMLAB_STATIC_DIR` — directory with built SPA `index.html` (e.g. `apps/web/dist`); missing/incomplete dir returns a 404 HTML page

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

## Install on Ubuntu (.deb) or Omarchy (.pkg.tar.zst)

The package installs `chemlab-server`, the built web UI, and a systemd unit that
starts on boot and binds **`127.0.0.1:3847`**. Put Caddy in front for HTTPS
(see below). `cargo run` defaults stay `127.0.0.1:3847`.

Build on the target distro (needs Rust, Node 22+, and the packager below).
`./update.sh` detects Ubuntu vs Omarchy and rebuilds + reinstalls.

**Ubuntu amd64** (needs `dpkg-deb`):

```bash
./scripts/build-deb.sh
sudo apt install ./dist/chemlab_*.deb
```

**Omarchy / Arch x86_64** (needs `makepkg` from `base-devel`):

```bash
./scripts/build-arch.sh
sudo pacman -U ./dist/chemlab-*.pkg.tar.zst
```

With Caddy in front, open `https://<host>/`. The daemon itself is not on the LAN
port. Direct health check on the host: `http://127.0.0.1:3847/api/health`.

| Path | Role |
| --- | --- |
| `/usr/bin/chemlab-server` | release binary |
| `/usr/share/chemlab/www/` | production web UI (`apps/web` `dist/`) |
| `/etc/chemlab/chemlab.env` | daemon env (not world-writable) |
| systemd unit (`User=chemlab`) | `/lib/systemd/system/chemlab.service` on Ubuntu; `/usr/lib/systemd/system/chemlab.service` on Omarchy |
| `/var/lib/chemlab/` | SQLite data dir |

The unit env is `CHEMLAB_BIND=127.0.0.1:3847`, `CHEMLAB_STATIC_DIR=/usr/share/chemlab/www`,
`CHEMLAB_DATABASE_URL=sqlite:///var/lib/chemlab/chemlab.db`,
`CHEMLAB_COOKIE_SECURE=true` (HTTPS via Caddy), and `CHEMLAB_SIGNUP_ENABLED=true`.
Debian `postinst` / Arch `post_install` enables and starts
the service; `prerm` / `pre_remove` stops and disables it on remove.

```bash
sudo systemctl status chemlab
sudo journalctl -u chemlab -e
```

### Caddy (HTTPS)

`packaging/caddy/Caddyfile` is the local/LAN HTTPS example: it reverse-proxies
`:443` (`tls internal`) to the loopback daemon and sets HSTS
(`Strict-Transport-Security: max-age=31536000; includeSubDomains`, no `preload`)
on that HTTPS site only — not on Axum HTML, so LAN HTTP is not HSTS-locked.

For a public hostname, use `packaging/caddy/Caddyfile.internet-facing` (Let's
Encrypt automatic HTTPS; replace `chemlab.example.com` with the real DNS name).
Do not run both Caddyfiles at once.

Install [Caddy](https://caddyserver.com/docs/install) (Ubuntu package, or on Omarchy
`omarchy pkg add caddy` / `sudo pacman -S caddy`), then:

```bash
caddy run --config packaging/caddy/Caddyfile
# public hostname:
# caddy run --config packaging/caddy/Caddyfile.internet-facing
```

On a packaged host you can copy the chosen file over `/etc/caddy/Caddyfile` and run
`sudo systemctl reload caddy` instead. Caddy is not bundled in the `.deb` or
`.pkg.tar.zst`.

With Caddy in front, ChemLab is not exposed on LAN port 3847. To serve HTTP on
the LAN without Caddy, set `CHEMLAB_BIND=0.0.0.0:3847` and
`CHEMLAB_COOKIE_SECURE=false` in `/etc/chemlab/chemlab.env` (or the repo
`packaging/common/chemlab.env` before you build) and restart `chemlab`.

CI uploads the `.deb` as `chemlab-deb` and the Arch package as `chemlab-arch`.

## Auth stub (accounts from day one)

Working thin stub (not a fake button):

| Method | Path | Notes |
| --- | --- | --- |
| `GET`  | `/api/auth/csrf` | issues HttpOnly `chemlab_csrf` cookie + `{ csrf_token }` |
| `POST` | `/api/auth/register` | email, password (≥8), display_name → sets session cookie; requires CSRF; rate-limited; 403 `{ code: "signup_disabled" }` when `CHEMLAB_SIGNUP_ENABLED` is false |
| `POST` | `/api/auth/login` | email, password → rotates session (invalidates previous, sets new cookie); requires CSRF; rate-limited |
| `POST` | `/api/auth/logout` | clears cookie + deletes server session; requires CSRF |
| `GET`  | `/api/auth/me` | `{ authenticated, user }` |
| `GET`  | `/api/lab/scene` | current lab snapshot; session required |
| `POST` | `/api/lab/action` | `{ type: use_tool \| pour, … }` → updated scene; session + CSRF |
| `POST` | `/api/lab/dissolve` | predict-only `{ substance_id, solvent_id, temperature_c }` → `{ dissolved, explanation }`; session + CSRF |

- Passwords: Argon2  
- Session cookie: `chemlab_session` (HttpOnly, SameSite=Lax; Secure when configured). Successful login issues a new session and invalidates the previous one.  
- CSRF: double-submit synchronizer — `GET /api/auth/csrf`, then send `X-CSRF-Token` matching `chemlab_csrf` on mutating POSTs (`/api/auth/*`, `/api/lab/action`, `/api/lab/dissolve` via `require_csrf`)  
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
build, an amd64 `.deb` package job, and an x86_64 Arch package job.

## Generate shared TS types

```bash
./scripts/generate-types.sh
```

v0.1 also keeps `apps/web/src/generated/contracts.ts` in sync by hand when needed; CI will tighten the dirty-check in a later milestone.

## Design notes

- Chemistry rules live in `chemlab-core` (server-authoritative). Browser never decides “did NaCl dissolve?”
- Login is required even for single-player (see project context).
- Signed-in UI is the 2D lab bench only; dissolve outcomes come from scene pour events (server-owned). No evaporate.

## License

MIT — see [LICENSE](LICENSE).
