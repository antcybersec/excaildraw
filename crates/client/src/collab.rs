use crate::crypto::RoomCipher;
use excaildraw_core::{ClientMessage, Element, ServerMessage};
use gloo::timers::callback::Timeout;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

const API_BASE: &str = "http://127.0.0.1:8080";

thread_local! {
    static AUTH_TOKEN: RefCell<Option<String>> = const { RefCell::new(None) };
    static ROOM_CIPHER: RefCell<Option<RoomCipher>> = const { RefCell::new(None) };
    static COLLAB: RefCell<Option<CollabHandle>> = const { RefCell::new(None) };
}

pub struct CollabHandle {
    ws: WebSocket,
}

pub fn set_auth_token(token: Option<String>) {
    AUTH_TOKEN.with(|t| *t.borrow_mut() = token);
}

pub fn set_room_cipher(cipher: Option<RoomCipher>) {
    ROOM_CIPHER.with(|c| *c.borrow_mut() = cipher);
}

impl CollabHandle {
    pub fn connect(
        room_id: &str,
        user: &str,
        on_sync: Rc<dyn Fn(Vec<Element>)>,
        on_cursor: Rc<dyn Fn(String, f64, f64, String)>,
    ) -> Result<Self, JsValue> {
        let url = format!("ws://127.0.0.1:8080/ws/{room_id}");
        let ws = WebSocket::new(&url)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let user_join = user.to_string();
        let color = crate::editor::user_color(user);
        let onopen_ws = ws.clone();
        let onopen = Closure::<dyn FnMut()>::new(move || {
            let join = ClientMessage::Join {
                user: user_join.clone(),
                color: color.clone(),
            };
            send_json(&onopen_ws, &join);
            wasm_bindgen_futures::spawn_local(async {
                flush_offline_queue().await;
            });
        });
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                let text: String = text.into();
                if let Ok(msg) = serde_json::from_str::<ServerMessage>(&text) {
                    match msg {
                        ServerMessage::Init { elements } | ServerMessage::Sync { elements } => {
                            on_sync(elements);
                        }
                        ServerMessage::Cursor { user, x, y, color } => {
                            on_cursor(user, x, y, color);
                        }
                        ServerMessage::Encrypted { payload } => {
                            ROOM_CIPHER.with(|c| {
                                if let Some(cipher) = c.borrow().as_ref() {
                                    if let Ok(inner) = cipher.decrypt(&payload) {
                                        if let Ok(ServerMessage::Sync { elements }) =
                                            serde_json::from_str::<ServerMessage>(&inner)
                                        {
                                            on_sync(elements);
                                        }
                                    }
                                }
                            });
                        }
                        ServerMessage::Error { .. } => {}
                    }
                }
            }
        });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        Ok(Self { ws })
    }

    pub fn send_update(&self, elements: &[Element]) {
        let msg = ClientMessage::Update {
            elements: elements.to_vec(),
        };
        if try_send_encrypted(&self.ws, &msg) {
            return;
        }
        send_json(&self.ws, &msg);
    }

    pub fn send_cursor(&self, user: &str, x: f64, y: f64) {
        let color = crate::editor::user_color(user);
        let msg = ClientMessage::Cursor {
            user: user.to_string(),
            x,
            y,
            color,
        };
        send_json(&self.ws, &msg);
    }
}

fn try_send_encrypted(ws: &WebSocket, msg: &ClientMessage) -> bool {
    ROOM_CIPHER.with(|c| {
        if let Some(cipher) = c.borrow().as_ref() {
            if let Ok(json) = serde_json::to_string(msg) {
                if let Ok(payload) = cipher.encrypt(&json) {
                    let enc = ClientMessage::Encrypted { payload };
                    send_json(ws, &enc);
                    return true;
                }
            }
        }
        false
    })
}

fn send_json(ws: &WebSocket, msg: &impl serde::Serialize) {
    if ws.ready_state() == WebSocket::OPEN {
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = ws.send_with_str(&json);
            return;
        }
    }
    if let Ok(json) = serde_json::to_string(msg) {
        wasm_bindgen_futures::spawn_local(async move {
            crate::storage::queue_update(&json).await;
        });
    }
}

pub fn set_collab(handle: CollabHandle) {
    COLLAB.with(|c| *c.borrow_mut() = Some(handle));
}

pub fn with_collab(f: impl FnOnce(&CollabHandle)) {
    COLLAB.with(|c| {
        if let Some(h) = c.borrow().as_ref() {
            f(h);
        }
    });
}

async fn flush_offline_queue() {
    let pending = crate::storage::drain_queue().await;
    COLLAB.with(|c| {
        if let Some(h) = c.borrow().as_ref() {
            for json in pending {
                let _ = h.ws.send_with_str(&json);
            }
        }
    });
}

pub async fn create_room() -> Option<String> {
    let token = AUTH_TOKEN.with(|t| t.borrow().clone());
    let mut builder = gloo_net::http::Request::post(&format!("{API_BASE}/api/rooms"));
    if let Some(token) = token {
        builder = builder.header("Authorization", &format!("Bearer {token}"));
    }
    let resp = builder.send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json["id"].as_str().map(str::to_string)
}

pub async fn fetch_auth_token(username: &str) -> Option<String> {
    let resp = gloo_net::http::Request::post(&format!("{API_BASE}/api/auth/token"))
        .header("Content-Type", "application/json")
        .body(serde_json::json!({ "username": username }).to_string())
        .ok()?
        .send()
        .await
        .ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json["token"].as_str().map(str::to_string)
}

pub fn throttle_cursor(user: &str, x: f64, y: f64) {
    thread_local! {
        static PENDING: RefCell<Option<(String, f64, f64)>> = const { RefCell::new(None) };
        static SCHEDULED: RefCell<bool> = const { RefCell::new(false) };
    }
    PENDING.with(|p| *p.borrow_mut() = Some((user.to_string(), x, y)));
    SCHEDULED.with(|s| {
        if *s.borrow() {
            return;
        }
        *s.borrow_mut() = true;
        Timeout::new(50, move || {
            SCHEDULED.with(|s| *s.borrow_mut() = false);
            PENDING.with(|p| {
                if let Some((user, x, y)) = p.borrow_mut().take() {
                    with_collab(|h| h.send_cursor(&user, x, y));
                }
            });
        })
        .forget();
    });
}

pub fn export_png(canvas: &web_sys::HtmlCanvasElement) -> Option<String> {
    canvas.to_data_url().ok()
}

pub fn download_file(filename: &str, content: &str, mime: &str) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let array = js_sys::Array::new();
    array.push(&JsValue::from_str(content));
    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type(mime);
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&array, &bag).unwrap();
    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
    let a = document.create_element("a").unwrap();
    let a: web_sys::HtmlAnchorElement = a.dyn_into().unwrap();
    a.set_download(filename);
    a.set_href(&url);
    a.click();
    let _ = web_sys::Url::revoke_object_url(&url);
}

pub fn download_data_url(filename: &str, data_url: &str) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let a = document.create_element("a").unwrap();
    let a: web_sys::HtmlAnchorElement = a.dyn_into().unwrap();
    a.set_download(filename);
    a.set_href(data_url);
    a.click();
}

pub fn persist_scene(json: String) {
    wasm_bindgen_futures::spawn_local(async move {
        crate::storage::save_scene(&json).await;
    });
}
