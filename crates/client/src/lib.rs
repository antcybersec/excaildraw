mod app;
mod canvas;
mod collab;
mod crypto;
mod editor;
mod icons;
mod render_loop;
mod storage;
mod theme;
mod viewport;

use app::App;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    yew::Renderer::<App>::new().render();
}
