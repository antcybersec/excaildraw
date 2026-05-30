use excaildraw_core::{Element, ElementType};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, MouseEvent, WheelEvent};
use yew::prelude::*;

use crate::canvas::render_scene;
use crate::viewport::Viewport;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rectangle,
    Ellipse,
    Freedraw,
}

#[function_component(App)]
pub fn app() -> Html {
    let canvas_ref = use_node_ref();
    let viewport = use_mut_ref(Viewport::default);
    let elements = use_mut_ref(Vec::<Element>::new);
    let tool = use_state(|| Tool::Rectangle);
    let drawing = use_mut_ref(|| None::<(f64, f64)>);
    let preview = use_mut_ref(|| None::<Element>);
    let panning = use_mut_ref(|| None::<(f64, f64)>);
    let freedraw_points = use_mut_ref(Vec::<f64>::new);

    let frame = use_state(|| 0u32);

    {
        let canvas_ref = canvas_ref.clone();
        let viewport = viewport.clone();
        let elements = elements.clone();
        let preview = preview.clone();
        let frame_id = *frame;
        use_effect_with(frame_id, move |_| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let rect = canvas.get_bounding_client_rect();
                let width = rect.width();
                let height = rect.height();
                canvas.set_width(width as u32);
                canvas.set_height(height as u32);
                if let Ok(Some(ctx)) = canvas
                    .get_context("2d")
                    .map(|c| c.map(|v| v.dyn_into::<CanvasRenderingContext2d>().ok()))
                    .map(|o| o.flatten())
                {
                    let vp = viewport.borrow();
                    let els = elements.borrow();
                    let prev = preview.borrow();
                    render_scene(
                        &ctx,
                        &vp,
                        width,
                        height,
                        &els,
                        prev.as_ref(),
                    );
                }
            }
            || ()
        });
    }

    let bump_frame = {
        let frame = frame.clone();
        Rc::new(move || frame.set(*frame + 1))
    };

    let on_wheel = {
        let viewport = viewport.clone();
        let canvas_ref = canvas_ref.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |e: WheelEvent| {
            e.prevent_default();
            let rect = canvas_ref
                .cast::<HtmlCanvasElement>()
                .map(|c| c.get_bounding_client_rect());
            if let Some(rect) = rect {
                let sx = e.client_x() as f64 - rect.left();
                let sy = e.client_y() as f64 - rect.top();
                viewport.borrow_mut().zoom_at(sx, sy, -e.delta_y());
                bump_frame();
            }
        })
    };

    let on_mouse_down = {
        let viewport = viewport.clone();
        let canvas_ref = canvas_ref.clone();
        let tool = tool.clone();
        let drawing = drawing.clone();
        let preview = preview.clone();
        let panning = panning.clone();
        let freedraw_points = freedraw_points.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |e: MouseEvent| {
            let rect = canvas_ref
                .cast::<HtmlCanvasElement>()
                .map(|c| c.get_bounding_client_rect());
            let Some(rect) = rect else { return };

            let sx = e.client_x() as f64 - rect.left();
            let sy = e.client_y() as f64 - rect.top();

            if e.button() == 1 || e.shift_key() {
                *panning.borrow_mut() = Some((sx, sy));
                return;
            }

            let (wx, wy) = viewport.borrow().screen_to_world(sx, sy);
            match *tool {
                Tool::Select => {}
                Tool::Rectangle => {
                    *drawing.borrow_mut() = Some((wx, wy));
                    *preview.borrow_mut() =
                        Some(Element::new(ElementType::Rectangle, wx, wy, 0.0, 0.0));
                }
                Tool::Ellipse => {
                    *drawing.borrow_mut() = Some((wx, wy));
                    *preview.borrow_mut() =
                        Some(Element::new(ElementType::Ellipse, wx, wy, 0.0, 0.0));
                }
                Tool::Freedraw => {
                    freedraw_points.borrow_mut().clear();
                    freedraw_points.borrow_mut().extend([wx, wy]);
                    let mut el = Element::new(ElementType::Freedraw, wx, wy, 0.0, 0.0);
                    el.points = Some(freedraw_points.borrow().clone());
                    *preview.borrow_mut() = Some(el);
                    *drawing.borrow_mut() = Some((wx, wy));
                }
            }
            bump_frame();
        })
    };

    let on_mouse_move = {
        let viewport = viewport.clone();
        let canvas_ref = canvas_ref.clone();
        let drawing = drawing.clone();
        let preview = preview.clone();
        let panning = panning.clone();
        let freedraw_points = freedraw_points.clone();
        let tool = tool.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |e: MouseEvent| {
            let rect = canvas_ref
                .cast::<HtmlCanvasElement>()
                .map(|c| c.get_bounding_client_rect());
            let Some(rect) = rect else { return };

            let sx = e.client_x() as f64 - rect.left();
            let sy = e.client_y() as f64 - rect.top();

            if let Some((px, py)) = *panning.borrow() {
                viewport.borrow_mut().pan(sx - px, sy - py);
                *panning.borrow_mut() = Some((sx, sy));
                return;
            }

            if drawing.borrow().is_none() {
                return;
            }

            let (wx, wy) = viewport.borrow().screen_to_world(sx, sy);
            match *tool {
                Tool::Rectangle | Tool::Ellipse => {
                    if let Some((sx0, sy0)) = *drawing.borrow() {
                        if let Some(p) = preview.borrow_mut().as_mut() {
                            p.x = sx0.min(wx);
                            p.y = sy0.min(wy);
                            p.width = (wx - sx0).abs();
                            p.height = (wy - sy0).abs();
                        }
                    }
                }
                Tool::Freedraw => {
                    freedraw_points.borrow_mut().extend([wx, wy]);
                    if let Some(p) = preview.borrow_mut().as_mut() {
                        p.points = Some(freedraw_points.borrow().clone());
                    }
                }
                Tool::Select => {}
            }
            bump_frame();
        })
    };

    let on_mouse_up = {
        let drawing = drawing.clone();
        let preview = preview.clone();
        let elements = elements.clone();
        let panning = panning.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |_: MouseEvent| {
            if panning.borrow_mut().take().is_some() {
                return;
            }
            if let Some(mut el) = preview.borrow_mut().take() {
                if el.width.abs() > 1.0 || el.height.abs() > 1.0 || el.points.is_some() {
                    el.bump_version();
                    elements.borrow_mut().push(el);
                }
            }
            drawing.borrow_mut().take();
            bump_frame();
        })
    };

    let set_tool = |t: Tool| {
        let tool = tool.clone();
        Callback::from(move |_| tool.set(t))
    };

    html! {
        <div class="app">
            <header class="toolbar">
                <span class="brand">{"excaildraw"}</span>
                <button class={classes!("tool-btn", (*tool == Tool::Select).then_some("active"))}
                    onclick={set_tool(Tool::Select)}>{"Select"}</button>
                <button class={classes!("tool-btn", (*tool == Tool::Rectangle).then_some("active"))}
                    onclick={set_tool(Tool::Rectangle)}>{"Rectangle"}</button>
                <button class={classes!("tool-btn", (*tool == Tool::Ellipse).then_some("active"))}
                    onclick={set_tool(Tool::Ellipse)}>{"Ellipse"}</button>
                <button class={classes!("tool-btn", (*tool == Tool::Freedraw).then_some("active"))}
                    onclick={set_tool(Tool::Freedraw)}>{"Draw"}</button>
                <span class="hint">{"Scroll to zoom · Shift+drag to pan"}</span>
            </header>
            <canvas
                ref={canvas_ref}
                class="canvas"
                onwheel={on_wheel}
                onmousedown={on_mouse_down}
                onmousemove={on_mouse_move}
                onmouseup={on_mouse_up.clone()}
                onmouseleave={on_mouse_up}
            />
        </div>
    }
}
