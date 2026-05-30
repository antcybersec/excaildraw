mod app;
mod canvas;
mod collab;
mod editor;
mod viewport;

use app::App;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    yew::Renderer::<App>::new().render();
}
