# rustCanvas

A scalable, Excalidraw-like collaborative whiteboard built in full-stack Rust.

- **Frontend:** Yew + WASM + HTML5 Canvas (Trunk) — dev on port **3000**
- **Backend:** Axum + Tokio + WebSockets on port **8080**
- **Production:** Single server serves WASM UI + API from one port
- **Shared core:** Excalidraw-compatible elements, JSON I/O, reconciliation, undo/redo, SVG export, rough sketch paths
- **Persistence:** PostgreSQL (optional) with in-memory fallback
- **Scaling:** Redis pub/sub (optional) for multi-instance WebSocket broadcast
- **Desktop:** Tauri 2 shell in `apps/desktop`

## Repository

https://github.com/antcybersec/excaildraw

## Features

- **Drawing tools:** select/move, rectangle, ellipse, line, arrow, freehand, text, image, eraser
- **Canvas:** infinite pan/zoom, light/dark theme, rough/sketch strokes, selection highlights
- **Layers panel:** reorder elements, select from list
- **History:** undo/redo (Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y)
- **Import/export:** `.excalidraw` JSON, SVG, PNG
- **Collaboration:** WebSocket rooms, live sync, remote cursors
- **E2E encryption:** optional room password (AES-GCM)
- **Offline:** IndexedDB scene cache + pending update queue
- **Auth:** optional JWT; set `REQUIRE_AUTH=1` to gate room creation

## Project structure

```
crates/
  core/     Shared types, history, hit-test, SVG export, protocol, rough renderer
  client/   WASM drawing UI, collab, crypto, IndexedDB offline
  server/   REST API, WebSocket sync, PostgreSQL, JWT, Redis broker
apps/
  desktop/  Tauri 2 native shell
docker/     Postgres + Redis compose + production Dockerfile
scripts/    build-release.sh for local production builds
```

## Prerequisites

- Rust 1.75+
- `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev/)
- Docker (optional, for PostgreSQL + Redis + production deploy)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | HTTP listen port |
| `FRONTEND_DIST` | `dist` | Path to Trunk `dist/` for serving the WASM UI |
| `DATABASE_URL` | *(none)* | PostgreSQL connection string |
| `REDIS_URL` | *(none)* | Redis URL for cross-instance broadcast |
| `JWT_SECRET` | dev secret | HMAC secret for JWT signing |
| `REQUIRE_AUTH` | *(off)* | Set to `1` to require Bearer token on `POST /api/rooms` |

## Development

### 1. Run API server

```bash
cargo run -p excaildraw-server
```

### 2. Run web client (separate terminal)

```bash
cd crates/client && trunk serve --open
```

Open http://127.0.0.1:3000

> Port **8080** is API-only in dev. The drawing UI is on **3000**.

### 3. Optional: Postgres + Redis

```bash
docker compose -f docker/docker-compose.yml up -d postgres redis
export DATABASE_URL=postgres://excaildraw:excaildraw@localhost:5432/excaildraw
export REDIS_URL=redis://localhost:6379
```

### Collaboration

1. Optionally set a **room password** (E2E encryption)
2. Click **Live collab** — creates a room and updates URL (`?room=...`)
3. Share the URL (and password if used)
4. Draw together — live sync + remote cursors

### Keyboard shortcuts

| Key | Tool |
|-----|------|
| 1–9 | Tools (eraser = E) |
| Scroll | Zoom |
| Space + drag | Pan |
| Shift | Constrain shapes |
| Ctrl/Cmd + Z | Undo |

## Production deployment

### Option A — Docker (recommended)

Builds WASM client + server into one image. Serves UI and API on port **8080**.

```bash
docker compose -f docker/docker-compose.yml up -d --build
```

Open http://localhost:8080

Includes Postgres + Redis for persistent rooms and horizontal scaling.

### Option B — Bare metal / VPS

```bash
./scripts/build-release.sh
FRONTEND_DIST=./dist ./target/release/excaildraw-server
```

Put **nginx** or **Caddy** in front for HTTPS:

```nginx
server {
    listen 443 ssl;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

WebSocket collab requires the `Upgrade` headers above.

### Option C — Static hosting + separate API

1. `cd crates/client && trunk build --release` → upload `dist/` to Cloudflare Pages / Netlify / S3
2. Deploy server to Fly.io / Railway / Render with `DATABASE_URL` + `REDIS_URL`
3. Set client API base via same-origin reverse proxy or configure CORS on server

### Pre-deploy checklist

- [ ] Set `JWT_SECRET` to a strong random value
- [ ] Set `DATABASE_URL` for room persistence
- [ ] Set `REDIS_URL` if running multiple server instances
- [ ] Build with `--release` (WASM + server)
- [ ] Enable HTTPS (required for secure WebSockets in production)
- [ ] Set `REQUIRE_AUTH=1` if you need gated room creation

## Tests

```bash
cargo test -p excaildraw-core -p excaildraw-server -p excaildraw-client
cargo clippy -p excaildraw-core -p excaildraw-server -p excaildraw-client --all-targets -- -D warnings
cd crates/client && trunk build --release
```

## License

MIT
