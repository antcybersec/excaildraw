# excaildraw

A scalable, Excalidraw-like collaborative drawing app built in full-stack Rust.

- **Frontend:** Yew + WASM + HTML5 Canvas (Trunk)
- **Backend:** Axum + Tokio
- **Shared core:** Excalidraw-compatible element model, JSON I/O, reconciliation

## Repository

https://github.com/antcybersec/excaildraw

## Project structure

```
crates/
  core/     Shared types, .excalidraw JSON, merge/reconcile logic
  client/   WASM drawing UI
  server/   REST API and (future) WebSocket collaboration
```

## Prerequisites

- Rust 1.75+ (`rustup`)
- `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev/) for the web client

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Development

### Run the API server

```bash
cargo run -p excaildraw-server
```

- Health: `GET http://localhost:8080/health`
- Create room: `POST http://localhost:8080/api/rooms`
- Get room: `GET http://localhost:8080/api/rooms/{id}`

### Run the web client

```bash
cd crates/client && trunk serve --open
```

Open http://127.0.0.1:8080 (Trunk default). Use rectangle, ellipse, and freehand tools. Scroll to zoom, Shift+drag to pan.

### Tests

```bash
cargo test --workspace
```

## Roadmap

See [excaildraw.md](./excaildraw.md) for architecture, scaling, and phased delivery plan.

## License

MIT
