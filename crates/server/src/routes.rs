use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use excaildraw_core::ExcalidrawFile;
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Serialize)]
struct CreateRoomResponse {
    id: String,
    url: String,
}

#[derive(Serialize)]
struct RoomResponse {
    id: String,
    file: ExcalidrawFile,
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/{id}", get(get_room))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "excaildraw-server",
    })
}

async fn create_room(State(state): State<AppState>) -> Json<CreateRoomResponse> {
    let id = state.create_room();
    Json(CreateRoomResponse {
        url: format!("/room/{id}"),
        id,
    })
}

async fn get_room(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<RoomResponse>, StatusCode> {
    state
        .get_room(&id)
        .map(|file| Json(RoomResponse { id, file }))
        .ok_or(StatusCode::NOT_FOUND)
}
