use excaildraw_core::ExcalidrawFile;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct AppState {
    inner: Arc<RwLock<RoomStore>>,
}

#[derive(Default)]
struct RoomStore {
    rooms: HashMap<String, ExcalidrawFile>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_room(&self) -> String {
        let id = Uuid::new_v4().to_string();
        let file = ExcalidrawFile::new(vec![]);
        self.inner.write().unwrap().rooms.insert(id.clone(), file);
        id
    }

    pub fn get_room(&self, id: &str) -> Option<ExcalidrawFile> {
        self.inner.read().unwrap().rooms.get(id).cloned()
    }
}
