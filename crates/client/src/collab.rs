use excaildraw_core::{ClientMessage, Element, ServerMessage};
use gloo::timers::callback::Timeout;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

const API_BASE: &str = "http://127.0.0.1:8080";

pub struct CollabHandle {
    ws: WebSocket,
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
            if let Ok(json) = serde_json::to_string(&join) {
                let _ = onopen_ws.send_with_str(&json);
            }
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
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = self.ws.send_with_str(&json);
        }
    }

    pub fn send_cursor(&self, user: &str, x: f64, y: f64) {
        let color = crate::editor::user_color(user);
        let msg = ClientMessage::Cursor {
            user: user.to_string(),
            x,
            y,
            color,
        };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = self.ws.send_with_str(&json);
        }
    }
}

thread_local! {
    static COLLAB: RefCell<Option<CollabHandle>> = const { RefCell::new(None) };
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

pub async fn create_room() -> Option<String> {
    let resp = gloo_net::http::Request::post(&format!("{API_BASE}/api/rooms"))
        .send()
        .await
        .ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    json["id"].as_str().map(str::to_string)
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
