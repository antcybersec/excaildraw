#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> Building WASM client (release)"
cd crates/client
trunk build --release
cd "$ROOT"

echo "==> Building API server (release)"
cargo build --release -p excaildraw-server

echo "==> Done"
echo "    UI + API: FRONTEND_DIST=$ROOT/dist ./target/release/excaildraw-server"
echo "    Open:     http://127.0.0.1:8080"
