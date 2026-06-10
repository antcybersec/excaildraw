use crate::auth::{bearer_user, issue_token, TokenRequest, TokenResponse};
use crate::broker::Broker;
use crate::db::Database;
use crate::state::AppState;
use crate::ws;
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use tower_http::services::{ServeDir, ServeFile};
use excaildraw_core::ExcalidrawFile;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct CreateRoomResponse {
    id: String,
    url: String,
    ws_url: String,
}

#[derive(Serialize)]
struct RoomResponse {
    id: String,
    file: ExcalidrawFile,
}

#[derive(Deserialize)]
struct SaveRoomRequest {
    file: ExcalidrawFile,
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/auth/token", post(auth_token))
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/{id}", get(get_room).put(save_room))
        .route("/ws/{id}", get(ws_upgrade))
}

fn frontend_dist() -> std::path::PathBuf {
    std::env::var("FRONTEND_DIST")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("dist"))
}

pub fn app_routes(state: AppState) -> Router {
    let api = api_routes().with_state(state);
    let dist = frontend_dist();
    let index = dist.join("index.html");

    if index.exists() {
        let serve_dir = ServeDir::new(&dist).not_found_service(ServeFile::new(index));
        api.fallback_service(serve_dir)
    } else {
        api.route("/", get(dev_landing))
    }
}

async fn dev_landing() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>excaildraw</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 640px; margin: 48px auto; padding: 0 16px; line-height: 1.5; }
    code, pre { background: #f1f3f5; border-radius: 6px; }
    code { padding: 2px 6px; }
    pre { padding: 12px; overflow-x: auto; }
    a { color: #6965db; }
  </style>
</head>
<body>
  <h1>excaildraw API is running</h1>
  <p>Port <strong>8080</strong> serves the backend only. The drawing UI is a separate WASM app.</p>
  <h2>Start the UI (dev)</h2>
  <p>In a <strong>second terminal</strong>:</p>
  <pre>cd crates/client && trunk serve --open</pre>
  <p>Then open <a href="http://127.0.0.1:3000">http://127.0.0.1:3000</a></p>
  <h2>Or serve UI from this port</h2>
  <p>Build once, then refresh this page:</p>
  <pre>cd crates/client && trunk build --release</pre>
  <p>That writes static files to <code>dist/</code>, which this server will serve automatically.</p>
</body>
</html>"#,
    )
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "excaildraw-server",
    })
}

async fn auth_token(Json(body): Json<TokenRequest>) -> Result<Json<TokenResponse>, StatusCode> {
    if body.username.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    issue_token(&body.username)
        .map(Json)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn create_room(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CreateRoomResponse>, StatusCode> {
    if std::env::var("REQUIRE_AUTH").ok().as_deref() == Some("1") {
        bearer_user(headers.get("authorization").and_then(|v| v.to_str().ok()))
            .ok_or(StatusCode::UNAUTHORIZED)?;
    }
    let id = state.create_room().await;
    Ok(Json(CreateRoomResponse {
        url: format!("/?room={id}"),
        ws_url: format!("/ws/{id}"),
        id,
    }))
}

async fn get_room(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RoomResponse>, StatusCode> {
    state
        .get_room(&id)
        .await
        .map(|file| Json(RoomResponse { id, file }))
        .ok_or(StatusCode::NOT_FOUND)
}

async fn save_room(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SaveRoomRequest>,
) -> Result<StatusCode, StatusCode> {
    if state.save_room(&id, body.file).await {
        Ok(StatusCode::OK)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if state.get_room(&id).await.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    ws.on_upgrade(move |socket| ws::handle_socket(socket, id, state))
        .into_response()
}

pub async fn build_state(broker: Broker) -> AppState {
    let db = Database::connect().await;
    AppState::new(db, broker)
}
