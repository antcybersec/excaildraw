use excaildraw_core::{reconcile_elements, ExcalidrawFile, ServerMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::db::Database;

pub type RoomSender = broadcast::Sender<ServerMessage>;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<RoomStore>>,
    db: Arc<Database>,
}

struct RoomStore {
    rooms: HashMap<String, RoomEntry>,
}

struct RoomEntry {
    file: ExcalidrawFile,
    tx: RoomSender,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RoomStore {
                rooms: HashMap::new(),
            })),
            db: Arc::new(db),
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
            let _ = entry.tx.send(ServerMessage::Sync {
                elements: merged.clone(),
            });
            let file = entry.file.clone();
            drop(store);
            self.db.save_room(id, &file).await;
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
