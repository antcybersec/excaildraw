# excaildraw

A scalable, Excalidraw-like collaborative drawing app built in full-stack Rust.

- **Frontend:** Yew + WASM + HTML5 Canvas (Trunk) on port **3000**
- **Backend:** Axum + Tokio + WebSockets on port **8080**
- **Shared core:** Excalidraw-compatible elements, JSON I/O, reconciliation, undo/redo, SVG export, rough sketch paths
- **Persistence:** PostgreSQL (optional via `DATABASE_URL`) with in-memory fallback
- **Scaling:** Redis pub/sub (optional via `REDIS_URL`) for multi-instance WebSocket broadcast
- **Desktop:** Tauri 2 shell in `apps/desktop`

## Repository

https://github.com/antcybersec/excaildraw

## Features

- **Drawing tools:** select/move (multi-select with Shift/Cmd), rectangle, ellipse, line, arrow, freehand, text, image
- **Canvas:** infinite pan/zoom, grid, rough/sketch stroke rendering, selection highlights
- **Layers panel:** reorder elements, select from list
- **History:** undo/redo (Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y)
- **Import/export:** `.excalidraw` JSON, SVG, PNG
- **Collaboration:** WebSocket rooms, live sync, remote cursors
- **E2E encryption:** optional room password — client encrypts sync payloads (AES-GCM); server relays ciphertext only
- **Offline:** IndexedDB scene cache + pending update queue when disconnected
- **Auth:** optional JWT (`POST /api/auth/token`); set `REQUIRE_AUTH=1` to gate room creation
- **Rate limiting:** in-memory per-IP limits on REST endpoints
- **API:** REST room create/load/save + WebSocket `/ws/{roomId}`

## Project structure

```
crates/
  core/     Shared types, history, hit-test, SVG export, protocol, rough renderer
  client/   WASM drawing UI, collab, crypto, IndexedDB offline
  server/   REST API, WebSocket sync, PostgreSQL, JWT, Redis broker
apps/
  desktop/  Tauri 2 native shell
docker/     Postgres + Redis compose + server Dockerfile
```

## Prerequisites

- Rust 1.75+
- `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev/)
- Docker (optional, for PostgreSQL + Redis)
- Node/npm (optional, only if building Tauri icons via `tauri icon`)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | *(none)* | PostgreSQL connection string; omit for in-memory rooms |
| `REDIS_URL` | *(none)* | Redis URL for cross-instance room broadcast |
| `JWT_SECRET` | dev secret | HMAC secret for JWT signing |
| `REQUIRE_AUTH` | *(off)* | Set to `1` to require Bearer token on `POST /api/rooms` |

## Development

### 1. Start PostgreSQL + Redis (optional)

```bash
docker compose -f docker/docker-compose.yml up -d
export DATABASE_URL=postgres://excaildraw:excaildraw@localhost:5432/excaildraw
export REDIS_URL=redis://localhost:6379
```

Without `DATABASE_URL`, rooms persist in memory only for the server process lifetime.

### 2. Run the API server

```bash
cargo run -p excaildraw-server
```

Endpoints:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| POST | `/api/auth/token` | Issue JWT (`{"username":"..."}`) |
| POST | `/api/rooms` | Create room |
| GET | `/api/rooms/{id}` | Load room |
| PUT | `/api/rooms/{id}` | Save room |
| WS | `/ws/{id}` | Real-time sync |

### 3. Run the web client

```bash
cd crates/client && trunk serve --open
```

Open http://127.0.0.1:3000

### 4. Desktop app (Tauri)

Build the WASM client first, then run Tauri from the desktop crate:

```bash
cd crates/client && trunk build --release
cd ../../apps/desktop/src-tauri && cargo tauri dev
```

Tauri loads the client from `http://127.0.0.1:3000` in dev or from `dist/` after `trunk build`.

### Collaboration workflow

1. Optionally enter a **room password** for end-to-end encrypted sync
2. Click **New Room** — creates a room and updates the URL (`?room=...`)
3. Share the URL (and password, if used) with others
4. Draw together — changes sync via WebSocket with reconciliation
5. Remote cursors appear with colored labels
6. Scene auto-saves to IndexedDB for offline reload

### Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| Scroll | Zoom |
| Shift + drag | Pan |
| Shift/Cmd + click | Add to selection |
| Ctrl/Cmd + Z | Undo |
| Ctrl/Cmd + Shift + Z | Redo |
| Ctrl/Cmd + Y | Redo |
| Ctrl/Cmd + S | Export JSON |
| Delete / Backspace | Delete selection |

### Tests

```bash
cargo test -p excaildraw-core -p excaildraw-server -p excaildraw-client
cargo clippy -p excaildraw-core -p excaildraw-server -p excaildraw-client --all-targets -- -D warnings
cd crates/client && trunk build --release
```

## Roadmap

See [excaildraw.md](./excaildraw.md) for full architecture. Future work:

- OAuth providers, NATS alternative to Redis
- Rough strokes in SVG export, async image rendering
- Yrs CRDT migration for conflict-free offline edits

## License

MIT
