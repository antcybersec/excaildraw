use axum::extract::ws::{Message, WebSocket};
use excaildraw_core::{ClientMessage, ServerMessage};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::state::AppState;

pub async fn handle_socket(socket: WebSocket, room_id: String, state: AppState) {
    let Some(tx) = state.room_sender(&room_id).await else {
        return;
    };

    let (write, mut read) = socket.split();
    let write = Arc::new(Mutex::new(write));
    let mut rx = tx.subscribe();

    let file = state
        .get_room(&room_id)
        .await
        .unwrap_or_else(|| excaildraw_core::ExcalidrawFile::new(vec![]));
    {
        let mut w = write.lock().await;
        let init = ServerMessage::Init {
            elements: file.elements,
        };
        if let Ok(json) = serde_json::to_string(&init) {
            let _ = w.send(Message::Text(json.into())).await;
        }
    }

    let write_broadcast = write.clone();
    let broadcast_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                let mut w = write_broadcast.lock().await;
                if w.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    while let Some(Ok(msg)) = read.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg {
                        ClientMessage::Update { elements } => {
                            state.apply_update(&room_id, elements).await;
                        }
                        ClientMessage::Cursor { user, x, y, color } => {
                            if let Some(tx) = state.room_sender(&room_id).await {
                                let _ = tx.send(ServerMessage::Cursor { user, x, y, color });
                            }
                        }
                        ClientMessage::Join { .. } => {}
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    broadcast_task.abort();
}
