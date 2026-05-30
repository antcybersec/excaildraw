use excaildraw_core::{reconcile_elements, ExcalidrawFile, ServerMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::broker::{Broker, BrokerEnvelope};
use crate::db::Database;

pub type RoomSender = broadcast::Sender<ServerMessage>;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<RoomStore>>,
    db: Arc<Database>,
    broker: Broker,
    instance_id: String,
}

struct RoomStore {
    rooms: HashMap<String, RoomEntry>,
}

struct RoomEntry {
    file: ExcalidrawFile,
    tx: RoomSender,
}

impl AppState {
    pub fn new(db: Database, broker: Broker) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RoomStore {
                rooms: HashMap::new(),
            })),
            db: Arc::new(db),
            broker,
            instance_id: Uuid::new_v4().to_string(),
        }
    }

    pub async fn create_room(&self) -> String {
        let id = Uuid::new_v4().to_string();
        let file = ExcalidrawFile::new(vec![]);
        self.insert_room(id.clone(), file).await;
        id
    }

    pub async fn get_room(&self, id: &str) -> Option<ExcalidrawFile> {
        {
            let store = self.inner.read().await;
            if let Some(entry) = store.rooms.get(id) {
                return Some(entry.file.clone());
            }
        }

        if let Some(file) = self.db.load_room(id).await {
            self.insert_room(id.to_string(), file.clone()).await;
            return Some(file);
        }
        None
    }

    pub async fn save_room(&self, id: &str, file: ExcalidrawFile) -> bool {
        self.update_room(id, file.clone()).await;
        self.db.save_room(id, &file).await;
        true
    }

    pub async fn room_sender(&self, id: &str) -> Option<RoomSender> {
        self.get_room(id).await?;
        self.inner.read().await.rooms.get(id).map(|r| r.tx.clone())
    }

    pub async fn apply_update(&self, id: &str, remote_elements: Vec<excaildraw_core::Element>) {
        let mut store = self.inner.write().await;
        if let Some(entry) = store.rooms.get_mut(id) {
            let merged = reconcile_elements(&entry.file.elements, &remote_elements);
            entry.file.elements = merged.clone();
            let msg = ServerMessage::Sync {
                elements: merged.clone(),
            };
            let _ = entry.tx.send(msg.clone());
            let file = entry.file.clone();
            drop(store);
            self.db.save_room(id, &file).await;
            self.broadcast_broker(id, &msg).await;
        }
    }

    pub async fn relay_encrypted(&self, id: &str, payload: String) {
        let msg = ServerMessage::Encrypted { payload: payload.clone() };
        if let Some(tx) = self.room_sender(id).await {
            let _ = tx.send(msg.clone());
        }
        self.broadcast_broker(id, &msg).await;
    }

    pub async fn forward_from_broker(&self, room_id: &str, envelope: BrokerEnvelope) {
        if envelope.instance_id == self.instance_id {
            return;
        }
        if let Ok(msg) = serde_json::from_str::<ServerMessage>(&envelope.payload) {
            if let Some(tx) = self.room_sender(room_id).await {
                let _ = tx.send(msg);
            }
        }
    }

    async fn broadcast_broker(&self, room_id: &str, msg: &ServerMessage) {
        if let Ok(payload) = serde_json::to_string(msg) {
            let envelope = BrokerEnvelope {
                instance_id: self.instance_id.clone(),
                payload,
            };
            self.broker.publish(room_id, &envelope).await;
        }
    }

    async fn insert_room(&self, id: String, file: ExcalidrawFile) {
        let (tx, _) = broadcast::channel(256);
        self.inner.write().await.rooms.insert(
            id.clone(),
            RoomEntry {
                file: file.clone(),
                tx,
            },
        );
        self.db.save_room(&id, &file).await;
    }

    async fn update_room(&self, id: &str, file: ExcalidrawFile) {
        let mut store = self.inner.write().await;
        if let Some(entry) = store.rooms.get_mut(id) {
            entry.file = file;
        } else {
            let (tx, _) = broadcast::channel(256);
            store.rooms.insert(
                id.to_string(),
                RoomEntry {
                    file: file.clone(),
                    tx,
                },
            );
        }
    }
}
