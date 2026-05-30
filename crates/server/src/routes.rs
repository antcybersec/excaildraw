use crate::db::Database;
use crate::state::AppState;
use crate::ws;
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
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
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/{id}", get(get_room).put(save_room))
        .route("/ws/{id}", get(ws_upgrade))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "excaildraw-server",
    })
}

async fn create_room(State(state): State<AppState>) -> Json<CreateRoomResponse> {
    let id = state.create_room().await;
    Json(CreateRoomResponse {
        url: format!("/?room={id}"),
        ws_url: format!("/ws/{id}"),
        id,
    })
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

pub async fn build_state() -> AppState {
    let db = Database::connect().await;
    AppState::new(db)
}
