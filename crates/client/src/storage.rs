use indexed_db_futures::prelude::*;
use wasm_bindgen::JsValue;

const DB_NAME: &str = "rustcanvas";
const STORE: &str = "kv";

async fn open_db() -> Result<IdbDatabase, JsValue> {
    let mut open_request = IdbDatabase::open_u32(DB_NAME, 1)?;
    open_request.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
        let db = evt.db();
        if !db.object_store_names().any(|n| n == STORE) {
            db.create_object_store(STORE)?;
        }
        Ok(())
    }));
    open_request.await.map_err(JsValue::from)
}

pub async fn idb_set(key: &str, value: &str) -> Result<(), JsValue> {
    let db = open_db().await?;
    let tx = db.transaction_on_one_with_mode(STORE, IdbTransactionMode::Readwrite)?;
    let store = tx.object_store(STORE)?;
    store.put_key_val_owned(key, &JsValue::from(value))?;
    tx.await.into_result().map_err(JsValue::from)
}

pub async fn idb_get(key: &str) -> Result<Option<String>, JsValue> {
    let db = open_db().await?;
    let tx = db.transaction_on_one(STORE)?;
    let store = tx.object_store(STORE)?;
    let val = store.get_owned(key)?.await?;
    tx.await.into_result().map_err(JsValue::from)?;
    Ok(val.and_then(|v| v.as_string()))
}

pub async fn save_scene(json: &str) {
    let _ = idb_set("scene", json).await;
}

pub async fn load_scene() -> Option<String> {
    idb_get("scene").await.ok().flatten()
}

pub async fn queue_update(json: &str) {
    let existing = idb_get("pending_queue").await.ok().flatten().unwrap_or_else(|| "[]".into());
    if let Ok(mut arr) = serde_json::from_str::<Vec<String>>(&existing) {
        arr.push(json.to_string());
        if let Ok(next) = serde_json::to_string(&arr) {
            let _ = idb_set("pending_queue", &next).await;
        }
    }
}

pub async fn drain_queue() -> Vec<String> {
    let existing = idb_get("pending_queue").await.ok().flatten();
    let _ = idb_set("pending_queue", "[]").await;
    existing
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
