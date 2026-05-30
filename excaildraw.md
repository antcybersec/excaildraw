# Executive Summary  

This report evaluates how to build a **scalable, Excalidraw-like collaborative drawing app** in full-stack Rust.  Key features include freehand drawing, shapes, text, layers, selection, undo/redo, multi-user real-time collaboration with presence/cursors, version history, and import/export.  We assume a **web-first** client (WASM) with optional desktop via Tauri. The recommended tech stack is: a Rust/WASM frontend (e.g. **Yew** or **Leptos**), a Rust backend service (e.g. **Axum** or **Actix Web**), and CRDT-based realtime sync (e.g. **Yjs/Yrs** or **Automerge-rs**).  Data is stored in PostgreSQL or CockroachDB (with Redis for caching/pubsub). Real-time comms use WebSockets; WebRTC is possible but complex. Conflict resolution leverages CRDT libraries (Yrs for shared types or Automerge-rs) to merge operations and support offline editing. We suggest *SerDe*-based JSON or binary (bincode/CBOR) for serialization of messages and state.  Deploy on containers with Kubernetes (or serverless) and use observability tools (OpenTelemetry + Prometheus) and CI/CD (GitHub Actions). Below is a **high-level architecture**:

```mermaid
flowchart LR
  subgraph Clients
    Browser1[User Browser (Rust/WASM UI)]
    Browser2
  end
  subgraph Backend
    LB[Load Balancer / Proxy]
    subgraph AppServerCluster
      App1[App Server (Axum/Actix)]
      App2[App Server (Axum/Actix)]
    end
    Broker[(NATS/Kafka)]
    DB[(PostgreSQL/CockroachDB)]
    Cache[(Redis)]
  end

  Browser1 -- WebSocket --> LB
  Browser2 -- WebSocket --> LB
  LB --> App1
  LB --> App2
  App1 -- SQL --> DB
  App1 -- pub/sub --> Broker
  App1 -- cache --> Cache
  App2 -- pub/sub --> Broker
```

**Fig.1:** Simplified system architecture (clients connect via WebSocket to a cluster of Rust backend servers, which use a broker and databases for scaling).  

## Project Goals & Scope  

- **Core features:** infinite canvas, freehand strokes, vector shapes (rectangles, ellipses, arrows, lines, text, etc.), layers/z-indexing, multi-select, align and distribute. Undo/redo is required.  
- **Collaboration:** real-time multi-user editing with presence (cursors, user colors) and version history.  Changes should merge correctly across concurrent editors.  
- **Persistence:** ability to save/load drawings (e.g. `.excalidraw` JSON), import/export (PNG/SVG). Local/offline support (IndexedDB) with sync when online.  
- **Platform:** Primarily web via WebAssembly. Optionally desktop via Tauri (embedding the same WASM UI).  
- **Scope:** Focus on real-time syncing (OT/CRDT), UI in Rust/WASM, backend in Rust. Assume no budget constraints but emphasize open-source, community-driven solutions.

## Performance & Scalability Requirements  

- **Latency:** For real-time collaboration, aim for end-to-end update latency <100ms across global regions (P50). Ideally leverage CDNs or edge servers.  
- **Concurrency:** Thousands of concurrent users per “room” may not be realistic; more likely tens. But the system should scale to many rooms in parallel. Each server should handle thousands of WebSocket connections (Tokio async, horizontal scaling via Kubernetes).  
- **Throughput:** Drawing operations (mouse strokes) produce many events (x,y points). Compress/aggregate them before broadcast. Use binary encoding (bincode/CBOR) for efficiency.  
- **Persistence:** Save states periodically or on demand. Support history/undo by storing deltas (event sourcing).  
- **Benchmarks:** Measure message round-trip time, ops/sec, and memory/CPU under load. Tools: wrk, k6, or custom Rust load test sending simulated CRDT updates.

## Architecture Options  

### Client-Server vs Peer-to-Peer  

- **Client-Server (Recommended):** Clients connect to a central server (WebSocket). The server sequences updates and broadcasts to peers. This simplifies discovery, storage, and moderation. It also centralizes conflict resolution.  
- **Peer-to-Peer (WebRTC):** Each client connects to others via WebRTC (mesh or via an SFU). This avoids a central relay but is complex to implement and scale. It may reduce latency for small groups but is hard for large rooms and lacks persistence. We recommend focusing on client-server with WebSockets【7†L53-L61】【11†L529-L537】.  

### Real-time Sync: OT vs CRDT  

- **Operational Transformation (OT):** Servers transform and rebroadcast operations (like Google Docs). OT is battle-tested for text but complex for custom shapes/trees. Rust has few mature OT libs.  
- **Conflict-Free Replicated Data Types (CRDTs, Recommended):** State-based or operation-based CRDTs let each client maintain a copy and merge without a central authority. This naturally supports offline editing and eventual consistency. Libraries like **Yjs/Yrs** or **Automerge-rs** provide rich data types with undo/redo, cursors, and snapshots【32†L327-L334】【39†L114-L122】.  
- Excalidraw’s team mentions they use a custom “reconciliation algorithm” for merge conflicts【11†L375-L384】. However, using an established CRDT can simplify development (and matches local-first goals). Yrs (a Rust port of Yjs) is high-performance and supports shared maps, arrays, text, etc.【39†L114-L122】【32†L327-L334】. Automerge-rs is another option (evolving, port of JS Automerge). The choice depends on interoperability and performance (Yrs is very fast; Automerge emphasizes ease of use).  

### System Layers  

- **Frontend (WASM UI):** Use a Rust WebAssembly framework (Yew, Leptos, Dioxus, Sycamore, etc.) to build the drawing UI. These compile to WASM and interact with the HTML5 Canvas via `web-sys` or libraries like `gloo`【7†L47-L55】.  
- **Backend (Server):** A Rust async web framework (Actix Web, Axum, or Warp) to handle HTTP (REST/gRPC) and WebSockets. These frameworks run on Tokio and support high concurrency. Axum is ergonomic and integrates well with Tokio (18k stars【53†L463-L471】); Actix Web is very mature and high-performance【53†L400-L408】.  
- **Real-time Hub:** WebSocket endpoints to manage rooms. Could use libraries like `tokio-tungstenite`, `warp::ws`, or Axum’s WebSocket extractor. Alternatively, use a high-level realtime framework (e.g. [`yomo-run`](https://github.com/yomorun/presencejs) for presence, as Excalidraw users suggest【11†L474-L481】).  
- **Data Flow:** Clients send drawing events or CRDT updates to server; server applies them to room state and broadcasts deltas to other clients. Server may also persist events or state to DB.  

## Data Models  

- **Shapes/Elements:** Model each drawable element with a struct (id, type, coordinates, style, text, etc.). Likely mirror Excalidraw’s JSON schema【24†L57-L66】 (id, type, x, y, width, height, other props). Each element has a unique ID (e.g. UUID or CRDT client id + counter).  
- **Events:** Two approaches:  
  - **Command Events:** E.g., “create shape X”, “update shape X at t=ts”, “delete shape Y”. These can be broadcast via WebSocket. With OT, events must be transformed. With CRDT, you might broadcast state changes.  
  - **CRDT Updates:** If using CRDT (Yrs/Automerge), then clients generate CRDT “updates” (binary blobs or JSON diffs) which the server relays to others. The server can also store these updates for late joiners.  
- **History/Versioning:** Store deltas (e.g. CRDT change logs or event log) in the database. Each room could have an append-only log to reconstruct history or snapshots for undo/redo. Alternatively, use CRDT’s built-in snapshots.  

## Networking & Protocols  

- **WebSocket:** The primary protocol for realtime sync. It supports bidirectional low-latency messages. WebSocket frames can carry JSON or binary payload (e.g. bincode/CBOR).  
- **WebRTC (Optional):** Peer-to-peer for video/audio or as backup data channel. Currently, Rust has [`webrtc-rs`](https://github.com/webrtc-rs) (Pion port) for media, but pure data channels are complex. For simplicity, WebRTC can be skipped or deferred.  
- **HTTP/REST API:** For non-real-time tasks (save/load session, auth, file export). Use a REST API on Axum/Actix to serve PNG/SVG exports, manage user sessions, etc.  
- **Message Formats:**  
  - *API Endpoints:* e.g. `POST /rooms` to create, `GET /rooms/{id}` to fetch session info.  
  - *WebSocket Messages:* Use a small JSON envelope or binary protocol. Example JSON messages:  
    ```json
    { "type": "join", "room": "123", "user": "Alice" }
    { "type": "cursor", "user": "Alice", "x": 100, "y": 150 }
    { "type": "draw", "payload": <binary CRDT diff> }
    { "type": "init", "elements": [...all shapes...], "appState": {...} }
    ```  
    Alternatively, use a binary format (Protocol Buffers or bincode-encoded struct) for efficiency. Serde can serialize Rust structs to JSON or CBOR.  

## Conflict Resolution  

- Use **CRDT** to automatically merge concurrent edits. For example, represent the drawing state as a CRDT document (maps of elements, arrays for order, etc.) so that insert/delete operations commute. Yrs provides maps/arrays and even text types, and generates *updates* that can be merged【39†L114-L122】.  
- If OT is used, implement transform logic manually (hard). CRDT is recommended for simpler reasoning and offline support.  
- **Undo/Redo:** Both Yjs and Automerge support undo/redo stacks out-of-the-box【32†L327-L334】. Use client-side undo by replaying CRDT history.  
- **Tombstones:** Deleting shapes can either use tombstones in CRDT or remove from map. Yrs and Automerge handle deletes deterministically.  

## Storage & Persistence  

- **Database:** Use PostgreSQL (with `tokio-postgres` or `sqlx`/`diesel`) or CockroachDB (Postgres-compatible, geo-distributed) to store persistent data: saved drawings, user accounts, room metadata. We recommend PostgreSQL for maturity, with an optional CockroachDB for global scale.  
- **Caching/Broker:** Use Redis (pub/sub) or NATS as a message broker to propagate events across server instances in a cluster. E.g., when a message arrives on App1, publish to a Redis channel for Room123; all instances subscribed relay to their clients in that room. NATS is lightweight and Rust-friendly (nats.rs); Kafka is heavy-weight but durable. For low-latency, Redis or NATS is ideal.  
- **Snapshots & Event Sourcing:** Periodically snapshot the whole canvas (JSON) in DB, and store incremental events/updates in an event log table. This allows replay (versioning) and crash recovery.  

## Offline Support & Sync  

- **Local-first Model:** Clients maintain local state (ex: in-memory CRDT or in IndexedDB). Use Service Workers and IndexedDB to persist state offline. Excalidraw does this for offline PWA【7†L57-L61】.  
- **Syncing:** On reconnect, client sends all local CRDT updates to server (or fetches missing updates via state vector diff as in Yrs). The server merges and broadcasts any new updates. Using CRDT makes this merge automatic.  
- **Conflict Handling:** As above, CRDT resolves conflicts. For complex cases (two users drawing on same shape), define an application rule (e.g. last-writer-wins or lock on selection).  

## Security  

- **Authentication/Authorization:** Use JWT tokens or OAuth for user identity. REST APIs check auth; WebSocket handshake includes auth token. Manage access so only authorized users can join a room (share link + token).  
- **Rate Limiting:** Apply rate limits on message frequency (to prevent abuse) using middleware or reverse proxy (e.g. nginx, Cloudflare). Throttle overly-frequent updates.  
- **Content Security:** Sanitize text elements to prevent XSS. Use HTTPS everywhere.  
- **Data Protection:** Consider end-to-end encryption for board state if needed (Excalidraw+ supports encrypted rooms). This is advanced; out-of-scope for initial implementation.  

## Testing & CI/CD  

- **Testing:** Write unit tests for core logic (shape model, history). Use integration tests simulating multiple clients (e.g. spawn multiple WebSocket connections using `tokio-tungstenite` in tests). Test offline/online sync flows.  
- **WASM Testing:** Use `wasm-pack test` for frontend logic, and `cargo test` for Rust code.  
- **CI/CD:** Use GitHub Actions (or GitLab CI) to build both backend and frontend. Steps: compile to WASM (via `wasm-pack` or Trunk), build Docker images, run tests. Automatic deployment pipelines (e.g. to Kubernetes or a serverless platform). Ensure code coverage and linting (`clippy`, `rustfmt`).  

## Deployment & Scaling  

- **Containers/Kubernetes:** Package the backend as a Docker container. Deploy in a Kubernetes cluster (e.g. GKE/EKS) with Horizontal Pod Autoscaling based on CPU or custom metrics (websocket connections). Use a Kubernetes Service or Ingress for load balancing (AWS ALB, GCP Load Balancer).  
- **Serverless:** The WASM frontend can be served from a static host (Netlify/Cloudflare Pages). Backend could be on serverless (AWS Fargate or Cloud Run), but WebSockets favor persistent servers. Alternatively, use Cloudflare Workers with Durable Objects for rooms (advanced).  
- **CI/CD:** Dockerfiles + Helm charts. Use tools like Terraform or Pulumi for infrastructure.  
- **Disaster Recovery:** Run multiple replicas in different zones. Use managed Postgres with read replicas. Use multiple regional instances of message broker.  

## Observability & Monitoring  

- **Logging:** Use structured logging (e.g. `tracing` crate) for Rust with output to JSON. Include request IDs.  
- **Metrics:** Expose Prometheus metrics (latency, active connections, message rates). Libraries: `prometheus` or `opentelemetry` crates. Integrate with Grafana dashboards.  
- **Tracing:** Use OpenTelemetry + Jaeger for distributed tracing of API calls and WebSocket events.  
- **Monitoring:** Set up alerts on error rates or high latency. Health-check endpoint for load balancer.  

## Developer Ergonomics  

- **Tooling:** Rust Analyzer (VSCode/IntelliJ) for IDE support. Use `cargo-watch` and hot-reload tools (e.g. Trunk or `cargo-leptos watch`) for faster feedback.  
- **WASM Dev:** Use `trunk` (for Yew) or `wasm-pack`/Webpack for bundling. Yew uses Trunk as recommended【45†L249-L252】. Leptos has `cargo-leptos` CLI (scaffolds full-stack with SSR)【45†L226-L234】.  
- **Debugging:** Print to JS console (`console_log` crate). Use browser dev tools for WASM debugging (source maps).  
- **Dependencies:** Use mature crates (Actix, Yew, Serde, etc.) and keep them updated.  

## Rust Ecosystem Choices  

Below we compare key Rust ecosystem options for each layer.

| **Layer**                   | **Options**                           | **Pros**                                                          | **Cons**                                                         | **Notes & Recommended**                       |
|-----------------------------|---------------------------------------|-------------------------------------------------------------------|------------------------------------------------------------------|-----------------------------------------------|
| **WASM Toolchain**          | *wasm-bindgen* / *wasm-pack*, *trunk* | Standard approach; wasm-pack bundles, trunk automates pipelines【45†L249-L252】. | wasm-bindgen is low-level; trunk has opinions (HTML template).   | Use **Trunk** (bundles HTML/CSS; standard for Yew)【45†L249-L252】. wasm-pack also acceptable. |
| **Frontend Framework**      | *Yew*, *Leptos*, *Dioxus*, *Sycamore*, *Seed* | Yew: mature, component model, 30.5k stars【46†L1-L4】; Sycamore: fine-grained reactivity (used in Perseus); Leptos: SSR+full-stack, 18.5k stars【49†L1-L4】; Dioxus: cross-platform, 20k stars【47†L1-L4】. | Yew: VDOM overhead; Leptos: new syntax, heavier SSR; Dioxus: heavier, virtual DOM; Sycamore: smaller community. | For **web-first**, **Yew** (30k stars) or **Sycamore** (for speed) are good. Yew has strong docs and ecosystem. Leptos is promising if SSR/islands needed (but not critical here). Seed is older Elm-like, less popular now. Dioxus is best if native desktop is a goal (with Tauri). |
| **Desktop GUI**            | *Tauri*                              | Leverage web UI for desktop; small binaries (native webview)【53†L350-L358】; cross-platform; 81k stars【53†L362-L364】. | Only for desktop, not web. Less mature than Electron but growing. | If desktop needed, **Tauri** is ideal【53†L350-L358】【53†L362-L364】 (uses any frontend framework). |
| **Rust Web Backend**       | *Actix Web*, *Axum*, *Warp*, *Rocket* | Actix: very mature, high perf, actor model, 23k stars【53†L400-L408】; Axum: ergonomic, async (18k stars)【53†L463-L471】; Warp: lightweight filters; Rocket: batteries-included, but was sync (now async). | Actix: steeper learning, complex generics; Warp: fewer features out-of-box; Rocket: slower update cycle. | **Axum** is recommended (simple async/Tokio, good WebSocket support) or **Actix** for performance-critical. |
| **CRDT Library**           | *yrs (Yjs port)*, *automerge-rs*, *crdts* | Yrs: high-perf CRDT with shared types, offline editing, undo/redo, snapshots【32†L327-L334】【39†L114-L122】; Automerge: easy JSON docs, active dev; crdts: simple CRDT primitives. | Yrs: larger dependency (Yjs-compat); Automerge-rs is maturing; crdts: lower-level. | **Yrs** (Yjs) is recommended for rich shared types and performance【32†L327-L334】【39†L114-L122】. Automerge-rs is alternative if JSON-compat. |
| **WebSocket Crate**        | *tokio-tungstenite*, *warp::ws*, *axum-websocket* | Many options; tokio-tungstenite is flexible; Axum and Warp have built-in support.  | All well-maintained. | Use whichever matches chosen framework (e.g. **Axum’s `Ws` extractor** or **Warp's `ws()`**). |
| **Serialization**          | *Serde (JSON)*, *bincode*, *ron*, *CBOR* | Serde JSON: readable, compatible; bincode/CBOR: compact binary. | JSON is verbose (but human-friendly); binary is more efficient.  | Use **Serde** with JSON for persistence; binary (bincode/CBOR) for WebSocket payloads (speed). |
| **Database**               | *PostgreSQL*, *Redis*, *CockroachDB*  | Postgres: robust, ACID, rich SQL, Rust drivers (sqlx/diesel). Redis: in-memory cache/pubsub. Cockroach: distributed SQL, cloud-friendly. | Postgres: single-site (unless Cockroach). Redis: not persistent (for cache only). | **PostgreSQL** (with Diesel or sqlx) + **Redis** for caching/pubsub. Consider **CockroachDB** if geo-replication needed. |
| **Message Broker**         | *NATS*, *Kafka*, *Redis Pub/Sub*      | NATS: lightweight pub/sub, easy Rust support. Kafka: durable but heavy. Redis Pub/Sub: simple, fast. | Kafka: operationally complex for this use. NATS or Redis suffice. | **NATS** is recommended for high-performance pub/sub. Redis Pub/Sub is acceptable for simpler needs. |
| **Deployment**             | *Kubernetes*, *Docker*, *Serverless*  | Kubernetes: scalable, works with containers (Docker). Serverless (Cloudflare Workers) for frontend; container for backend. | K8s has ops overhead; Serverless may complicate stateful WS. | Use **Docker + Kubernetes** (or managed container service). Serve WASM statically via CDN. |
| **CI/CD/Tooling**          | *GitHub Actions*, *Docker*, *Terraform* | Common cloud dev tools. | - | Use GitHub Actions for builds/tests. Docker for images. IaC (Terraform/Helm) for infra. |

## System Architecture (Mermaid)

```mermaid
flowchart LR
  subgraph User
    Browser["Browser (Rust WASM UI)"]
  end
  subgraph Server
    LB["Load Balancer\n(HTTPS, WS)"]
    subgraph Backend Cluster
      App[App Servers\n(Axum/Actix, WebSocket handlers)]
      App2[App Servers\n(Axum/Actix, WebSocket handlers)]
    end
    Broker[(NATS)]
    DB[(PostgreSQL)]
    Cache[(Redis)]
  end

  Browser -->|HTTPS / WebSocket| LB
  LB --> App
  LB --> App2
  App -- pub/sub ---> Broker
  App2 -- pub/sub ---> Broker
  App -- SQL / State ---> DB
  App2 -- SQL / State ---> DB
  App -- cache / PubSub ---> Cache
  App2 -- cache / PubSub ---> Cache
```

**Fig.2:** High-level system architecture. Clients (WASM in browser) connect via a load-balanced WebSocket/HTTPS gateway to backend servers. Servers coordinate via a message broker (NATS) and use a SQL database plus cache for persistence and scaling.  

## Development Roadmap  

| **Milestone**                | **Prototype (0–3 months)**                            | **Production-Ready (3–6+ months)**            |
|------------------------------|-------------------------------------------------------|----------------------------------------------|
| **UI & Canvas**              | Build basic drawing canvas in WASM (Yew/Leptos). Implement freehand and shape tools with Canvas API【7†L47-L55】. | Refine UI/UX, add layers and selection box, support images/fonts, responsive design. |
| **Local State & Undo/Redo**  | Maintain element list and state in-memory; implement undo/redo stack. | Persist history in IndexedDB; complex undo (group strokes), version snapshots. |
| **Collaboration (Basic)**    | WebSocket server that relays raw drawing commands (e.g., strokes) to all clients in room. No conflict resolution (single writer) or naive OT. | Integrate CRDT library (Yrs) on client+server to sync element states with merge. Support simultaneous edits. |
| **Presence & Cursors**       | Broadcast simple cursor-position messages over WS. | Show user avatars/cursors. Optimize messages (throttling). |
| **Persistence**              | Save/load whole drawing via REST (JSON). Allow export (PNG/SVG). | Incremental save: log events or CRDT updates in DB for recovery and versioning. |
| **Authentication**           | (Optional) Skip (anonymous rooms). Use shareable room IDs. | Implement user accounts (OAuth/JWT) and access control per room. |
| **Offline Support**          | Basic fallback (PWA install, cache). | Full offline mode: queue operations in IndexedDB, sync on reconnect via CRDT diff. |
| **Performance & Scale**      | Run on local machine or single server. Test with 2–5 users/room. | Load test (100+ concurrent users). Deploy multiple servers with Redis/NATS for PubSub. Optimize serialization. |
| **Testing & CI**             | Unit tests for core logic, manual multi-client testing. | Automated end-to-end tests, CI pipeline with WASM builds, cross-browser tests. |
| **Monitoring & Security**    | Logging, minimal error handling. | Add metrics (Prometheus), tracing. Harden auth, DDoS/rate-limit, TLS everywhere. |

## Example Tech Stack and API Design  

- **Frontend:** Rust + WASM (e.g. **Yew**), Canvas via `web-sys` or `gloo`. Build with **Trunk** or `wasm-pack`【45†L249-L252】.  
- **Backend:** Rust **Axum** or **Actix Web** server. WebSockets for `/ws/:roomId`. REST endpoints (e.g. `GET /api/rooms/{id}`, `POST /api/rooms`, `POST /api/rooms/{id}/save`).  
- **CRDT:** **Yrs** on both client (via `wasm-bindgen`) and server. Use `Doc` and shared maps for elements.  
- **DB:** PostgreSQL via `sqlx` or `diesel`; store room metadata and snapshots. Redis for caching and WS broadcast channels.  
- **Broker:** NATS (Rust crate `async_nats`) for inter-server pub/sub of WS messages by room.  
- **Serialization:** Define message structs (derive Serde). E.g.:  
  ```rust
  #[derive(Serialize, Deserialize)]
  struct UpdateMsg { room: String, user: String, update: Vec<u8> }
  #[derive(Serialize, Deserialize)]
  struct CursorMsg { room: String, user: String, x: f64, y: f64 }
  ```  
  WebSocket payload example: `{"type":"Update","data":<base64 CRDT diff>}`.  
- **Endpoints:**  
  - `POST /api/rooms` → create new room (returns ID).  
  - `GET /api/rooms/{id}` → get room info (optional).  
  - WebSocket `ws://server/ws/{room}`: on connect, server sends initial state, then bidirectional updates:  
    - `{"type":"Join","user":"Alice"}`  
    - `{"type":"Cursor","user":"Alice","x":100,"y":150}`  
    - `{"type":"Draw","ops": <binary CRDT ops>}`  
- **Message Format:** Use either text (JSON with base64) or binary frames (protobuf or bincode). For performance, binary frames carrying compact CRDT diffs are best. 

## Observability and Metrics  

Measure and monitor:  
- **Connections:** number of active WebSocket clients (per server, per room).  
- **Latency:** round-trip times of update messages (measure ping/pong).  
- **Throughput:** operations per second (updates transmitted) per room.  
- **Server Load:** CPU/memory per instance under load tests.  
- **Error Rates:** any dropped messages or failed syncs.  

Define alerts (e.g., connection failures >5%).

## Open Questions / Limitations  

- **CRDT Selection:** The ideal CRDT library depends on stability/performance at 2026. While Yrs is promising【39†L114-L122】, it must interoperate with other clients if we allow JS/Excalidraw clients. If compatibility is needed, using a JS-friendly format (e.g. Yjs or Automerge) is important.  
- **WebRTC:** We assume using WebSockets. True P2P (WebRTC) might reduce latency for few users but complicates the architecture. Evaluate later for scalable video+audio or offline mesh networks.  
- **Conflict Semantics:** If two users edit the same object (e.g. both move a shape), define policy (e.g. last-writer-wins in CRDT, or lock on drag).  
- **Encryption:** End-to-end encryption (like Excalidraw+ offers) would require encrypting CRDT updates. This is non-trivial and not covered here.  

**Sources:** Excalidraw documentation and discussions【3†L398-L404】【11†L375-L384】, Rust ecosystem blogs【45†L231-L239】【53†L350-L358】, and official CRDT docs【32†L327-L334】【39†L114-L122】 were referenced. These informed the feature list, technology comparisons, and architecture suggestions.  

