# excaildraw

A scalable, Excalidraw-like collaborative drawing app built in full-stack Rust.

- **Frontend:** Yew + WASM + HTML5 Canvas (Trunk) on port **3000**
- **Backend:** Axum + Tokio + WebSockets on port **8080**
- **Shared core:** Excalidraw-compatible elements, JSON I/O, reconciliation, undo/redo, SVG export
- **Persistence:** PostgreSQL (optional via `DATABASE_URL`) with in-memory fallback

## Repository

https://github.com/antcybersec/excaildraw

## Features

- **Drawing tools:** select/move, rectangle, ellipse, line, arrow, freehand, text
- **Canvas:** infinite pan/zoom, grid, selection highlights
- **History:** undo/redo (Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y)
- **Import/export:** `.excalidraw` JSON, SVG, PNG
- **Collaboration:** WebSocket rooms, live sync, remote cursors
- **API:** REST room create/load/save + WebSocket `/ws/{roomId}`

## Project structure

```
crates/
  core/     Shared types, history, hit-test, SVG export, protocol
  client/   WASM drawing UI + collaboration client
  server/   REST API, WebSocket sync, PostgreSQL persistence
docker/     Postgres compose + server Dockerfile
```

## Prerequisites

- Rust 1.75+
- `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev/)
- Docker (optional, for PostgreSQL)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Development

### 1. Start PostgreSQL (optional)

```bash
docker compose -f docker/docker-compose.yml up -d
export DATABASE_URL=postgres://excaildraw:excaildraw@localhost:5432/excaildraw
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
| POST | `/api/rooms` | Create room |
| GET | `/api/rooms/{id}` | Load room |
| PUT | `/api/rooms/{id}` | Save room |
| WS | `/ws/{id}` | Real-time sync |

### 3. Run the web client

```bash
cd crates/client && trunk serve --open
```

Open http://127.0.0.1:3000

### Collaboration workflow

1. Click **New Room** — creates a room and updates the URL (`?room=...`)
2. Share the URL with others
3. Draw together — changes sync via WebSocket with CRDT-style reconciliation
4. Remote cursors appear with colored labels

### Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| Scroll | Zoom |
| Shift + drag | Pan |
| Ctrl/Cmd + Z | Undo |
| Ctrl/Cmd + Shift + Z | Redo |
| Ctrl/Cmd + Y | Redo |
| Ctrl/Cmd + S | Export JSON |
| Delete / Backspace | Delete selection |

### Tests

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Roadmap

See [excaildraw.md](./excaildraw.md) for full architecture. Remaining production items:

- End-to-end encryption, OAuth auth, Redis/NATS horizontal scaling
- Rough/sketch rendering (rough.js style), image elements, layers UI
- IndexedDB offline queue, Tauri desktop shell

## License

MIT
