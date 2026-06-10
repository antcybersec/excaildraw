use crate::canvas::render_scene;
use crate::editor::Collaborators;
use crate::theme::CanvasTheme;
use crate::viewport::Viewport;
use excaildraw_core::Element;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::NodeRef;

pub fn start(
    canvas_ref: NodeRef,
    viewport: Rc<RefCell<Viewport>>,
    editor: Rc<RefCell<crate::editor::Editor>>,
    preview: Rc<RefCell<Option<Element>>>,
    collaborators: Rc<RefCell<Collaborators>>,
    theme: Rc<RefCell<CanvasTheme>>,
) -> impl FnOnce() {
    let running = Rc::new(Cell::new(true));
    let running_cb = running.clone();
    let closure = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let closure_cb = closure.clone();

    *closure.borrow_mut() = Some(Closure::new(move || {
        if !running_cb.get() {
            return;
        }

        viewport.borrow_mut().tick(1.0 / 60.0);

        if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
            let rect = canvas.get_bounding_client_rect();
            let css_w = rect.width();
            let css_h = rect.height();
            if css_w > 0.0 && css_h > 0.0 {
                let dpr = web_sys::window()
                    .map(|w| w.device_pixel_ratio())
                    .unwrap_or(1.0);
                let buf_w = (css_w * dpr).round() as u32;
                let buf_h = (css_h * dpr).round() as u32;
                if canvas.width() != buf_w || canvas.height() != buf_h {
                    canvas.set_width(buf_w);
                    canvas.set_height(buf_h);
                }

                if let Ok(Some(ctx)) = canvas
                    .get_context("2d")
                    .map(|c| c.map(|v| v.dyn_into::<CanvasRenderingContext2d>().ok()))
                    .map(|o| o.flatten())
                {
                    ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0).ok();
                    let ed = editor.borrow();
                    let prev = preview.borrow();
                    let collab = collaborators.borrow();
                    let theme = theme.borrow();
                    let selected: Vec<String> = ed.selected.iter().cloned().collect();
                    render_scene(
                        &ctx,
                        &viewport.borrow(),
                        css_w,
                        css_h,
                        &theme,
                        &ed.elements,
                        &ed.files,
                        prev.as_ref(),
                        &selected,
                        &collab,
                    );
                }
            }
        }

        if let Some(window) = web_sys::window() {
            if let Some(c) = closure_cb.borrow().as_ref() {
                let _ = window.request_animation_frame(c.as_ref().unchecked_ref());
            }
        }
    }));

    if let Some(window) = web_sys::window() {
        if let Some(c) = closure.borrow().as_ref() {
            let _ = window.request_animation_frame(c.as_ref().unchecked_ref());
        }
    }

    move || {
        running.set(false);
        drop(closure.borrow_mut().take());
    }
}
