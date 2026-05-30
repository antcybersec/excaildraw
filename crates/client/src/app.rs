use crate::canvas::render_scene;
use crate::collab::{
    create_room, download_data_url, download_file, export_png, set_collab, throttle_cursor,
    with_collab, CollabHandle,
};
use crate::editor::{Editor, Tool};
use crate::viewport::Viewport;
use excaildraw_core::{Element, ElementType, ExcalidrawFile, to_svg};
use gloo::events::EventListener;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement, KeyboardEvent, MouseEvent, WheelEvent};
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let canvas_ref = use_node_ref();
    let file_input_ref = use_node_ref();
    let viewport = use_mut_ref(Viewport::default);
    let editor = use_mut_ref(Editor::default);
    let tool = use_state(|| Tool::Select);
    let drawing = use_mut_ref(|| None::<(f64, f64)>);
    let preview = use_mut_ref(|| None::<Element>);
    let panning = use_mut_ref(|| None::<(f64, f64)>);
    let freedraw_points = use_mut_ref(Vec::<f64>::new);
    let collaborators = use_mut_ref(std::collections::HashMap::<String, (f64, f64, String)>::new);
    let room_id = use_state(String::new);
    let username = use_state(|| format!("User-{}", &uuid::Uuid::new_v4().to_string()[..4]));
    let frame = use_state(|| 0u32);
    let status = use_state(String::new);

    let bump_frame = {
        let frame = frame.clone();
        Rc::new(move || frame.set(*frame + 1))
    };

    {
        let canvas_ref = canvas_ref.clone();
        let viewport = viewport.clone();
        let editor = editor.clone();
        let preview = preview.clone();
        let collaborators = collaborators.clone();
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
                    let ed = editor.borrow();
                    let prev = preview.borrow();
                    let collab = collaborators.borrow();
                    render_scene(
                        &ctx,
                        &viewport.borrow(),
                        width,
                        height,
                        &ed.elements,
                        prev.as_ref(),
                        ed.selected.as_deref(),
                        &collab,
                    );
                }
            }
            || ()
        });
    }

    {
        let editor = editor.clone();
        let collaborators = collaborators.clone();
        let bump_frame = bump_frame.clone();
        let room_id = room_id.clone();
        let username = username.clone();
        let status = status.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let search = window.location().search().unwrap_or_default();
            if let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) {
                if let Some(room) = params.get("room") {
                    room_id.set(room.clone());
                    status.set(format!("Room: {room}"));
                    let editor_c = editor.clone();
                    let collaborators_c = collaborators.clone();
                    let user = (*username).clone();
                    let bump_sync = bump_frame.clone();
                    let bump_cursor = bump_frame.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Ok(handle) = CollabHandle::connect(
                            &room,
                            &user,
                            Rc::new(move |elements| {
                                editor_c.borrow_mut().merge_remote(elements);
                                bump_sync();
                            }),
                            Rc::new(move |user, x, y, color| {
                                collaborators_c.borrow_mut().insert(user, (x, y, color));
                                bump_cursor();
                            }),
                        ) {
                            set_collab(handle);
                        }
                    });
                }
            }
            || ()
        });
    }

    {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let listener = EventListener::new(&window, "keydown", move |e| {
                let e = e.dyn_ref::<KeyboardEvent>().unwrap();
                let key = e.key();
                if e.ctrl_key() || e.meta_key() {
                    match key.as_str() {
                        "z" if e.shift_key() => {
                            if editor.borrow_mut().redo() {
                                bump_frame();
                            }
                        }
                        "z" => {
                            if editor.borrow_mut().undo() {
                                bump_frame();
                            }
                        }
                        "y" => {
                            if editor.borrow_mut().redo() {
                                bump_frame();
                            }
                        }
                        "s" => {
                            e.prevent_default();
                            let file = ExcalidrawFile::new(editor.borrow().elements.clone());
                            if let Ok(json) = file.to_json_pretty() {
                                download_file("drawing.excalidraw", &json, "application/json");
                            }
                        }
                        _ => {}
                    }
                } else if (key == "Delete" || key == "Backspace")
                    && editor.borrow_mut().delete_selected()
                {
                    bump_frame();
                }
            });
            move || drop(listener)
        });
    }

    let on_wheel = {
        let viewport = viewport.clone();
        let canvas_ref = canvas_ref.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |e: WheelEvent| {
            e.prevent_default();
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let rect = canvas.get_bounding_client_rect();
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
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        let username = username.clone();
        Callback::from(move |e: MouseEvent| {
            let Some((wx, wy)) = (|| {
                let rect = canvas_ref.cast::<HtmlCanvasElement>()?.get_bounding_client_rect();
                let sx = e.client_x() as f64 - rect.left();
                let sy = e.client_y() as f64 - rect.top();
                Some(viewport.borrow().screen_to_world(sx, sy))
            })() else {
                return;
            };

            if e.button() == 1 || (e.button() == 0 && e.shift_key()) {
                let rect = canvas_ref.cast::<HtmlCanvasElement>().unwrap().get_bounding_client_rect();
                *panning.borrow_mut() = Some((e.client_x() as f64 - rect.left(), e.client_y() as f64 - rect.top()));
                return;
            }

            match *tool {
                Tool::Select => {
                    editor.borrow_mut().start_drag(wx, wy);
                }
                Tool::Rectangle => {
                    *drawing.borrow_mut() = Some((wx, wy));
                    *preview.borrow_mut() = Some(Element::new(ElementType::Rectangle, wx, wy, 0.0, 0.0));
                }
                Tool::Ellipse => {
                    *drawing.borrow_mut() = Some((wx, wy));
                    *preview.borrow_mut() = Some(Element::new(ElementType::Ellipse, wx, wy, 0.0, 0.0));
                }
                Tool::Line | Tool::Arrow => {
                    *drawing.borrow_mut() = Some((wx, wy));
                    let t = if *tool == Tool::Arrow {
                        ElementType::Arrow
                    } else {
                        ElementType::Line
                    };
                    let mut el = Element::new(t, wx, wy, 0.0, 0.0);
                    el.points = Some(vec![wx, wy, wx, wy]);
                    *preview.borrow_mut() = Some(el);
                }
                Tool::Freedraw => {
                    freedraw_points.borrow_mut().clear();
                    freedraw_points.borrow_mut().extend([wx, wy]);
                    let mut el = Element::new(ElementType::Freedraw, wx, wy, 0.0, 0.0);
                    el.points = Some(freedraw_points.borrow().clone());
                    *preview.borrow_mut() = Some(el);
                    *drawing.borrow_mut() = Some((wx, wy));
                }
                Tool::Text => {
                    if let Some(text) = web_sys::window().and_then(|w| w.prompt_with_message("Enter text:").ok().flatten()) {
                        if !text.is_empty() {
                            let mut el = Element::new(ElementType::Text, wx, wy, 100.0, 30.0);
                            el.text = Some(text);
                            el.font_size = Some(20.0);
                            el.bump_version();
                            editor.borrow_mut().push_element(el);
                            with_collab(|h| h.send_update(&editor.borrow().elements));
                        }
                    }
                }
            }
            throttle_cursor(&username, wx, wy);
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
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        let username = username.clone();
        Callback::from(move |e: MouseEvent| {
            let rect = canvas_ref.cast::<HtmlCanvasElement>().map(|c| c.get_bounding_client_rect());
            let Some(rect) = rect else { return };
            let sx = e.client_x() as f64 - rect.left();
            let sy = e.client_y() as f64 - rect.top();

            if let Some((px, py)) = *panning.borrow() {
                viewport.borrow_mut().pan(sx - px, sy - py);
                *panning.borrow_mut() = Some((sx, sy));
                bump_frame();
                return;
            }

            let (wx, wy) = viewport.borrow().screen_to_world(sx, sy);

            if editor.borrow().dragging.is_some() {
                editor.borrow_mut().drag_to(wx, wy);
                bump_frame();
                return;
            }

            if drawing.borrow().is_none() {
                throttle_cursor(&username, wx, wy);
                return;
            }

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
                Tool::Line | Tool::Arrow => {
                    if let Some((sx0, sy0)) = *drawing.borrow() {
                        if let Some(p) = preview.borrow_mut().as_mut() {
                            p.points = Some(vec![sx0, sy0, wx, wy]);
                        }
                    }
                }
                Tool::Freedraw => {
                    freedraw_points.borrow_mut().extend([wx, wy]);
                    if let Some(p) = preview.borrow_mut().as_mut() {
                        p.points = Some(freedraw_points.borrow().clone());
                    }
                }
                _ => {}
            }
            throttle_cursor(&username, wx, wy);
            bump_frame();
        })
    };

    let on_mouse_up = {
        let drawing = drawing.clone();
        let preview = preview.clone();
        let editor = editor.clone();
        let panning = panning.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |_: MouseEvent| {
            if panning.borrow_mut().take().is_some() {
                bump_frame();
                return;
            }
            if editor.borrow().dragging.is_some() {
                editor.borrow_mut().end_drag();
                with_collab(|h| h.send_update(&editor.borrow().elements));
                bump_frame();
                return;
            }
            if let Some(mut el) = preview.borrow_mut().take() {
                let valid = el.width.abs() > 1.0
                    || el.height.abs() > 1.0
                    || el.points.as_ref().is_some_and(|p| p.len() >= 4);
                if valid {
                    el.bump_version();
                    editor.borrow_mut().push_element(el);
                    with_collab(|h| h.send_update(&editor.borrow().elements));
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

    let on_create_room = {
        let room_id = room_id.clone();
        let status = status.clone();
        let editor = editor.clone();
        let collaborators = collaborators.clone();
        let bump_frame = bump_frame.clone();
        let username = username.clone();
        Callback::from(move |_| {
            let room_id = room_id.clone();
            let status = status.clone();
            let editor = editor.clone();
            let collaborators = collaborators.clone();
            let user = (*username).clone();
            let bump_sync = bump_frame.clone();
            let bump_cursor = bump_frame.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(id) = create_room().await {
                    room_id.set(id.clone());
                    status.set(format!("Room: {id}"));
                    if let Ok(handle) = CollabHandle::connect(
                        &id,
                        &user,
                        Rc::new(move |elements| {
                            editor.borrow_mut().merge_remote(elements);
                            bump_sync();
                        }),
                        Rc::new(move |u, x, y, color| {
                            collaborators.borrow_mut().insert(u, (x, y, color));
                            bump_cursor();
                        }),
                    ) {
                        set_collab(handle);
                    }
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_search(&format!("?room={id}"));
                    }
                }
            });
        })
    };

    let on_import = {
        let file_input_ref = file_input_ref.clone();
        Callback::from(move |_| {
            if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                input.click();
            }
        })
    };

    let on_file_change = {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(file) = input.files().and_then(|f| f.get(0)) {
                let editor = editor.clone();
                let bump = bump_frame.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let blob = gloo_file::Blob::from(file);
                    if let Ok(text) = gloo_file::futures::read_as_text(&blob).await {
                        if let Ok(parsed) = ExcalidrawFile::from_json(&text) {
                            editor.borrow_mut().set_elements(parsed.elements);
                            bump();
                        }
                    }
                });
            }
        })
    };

    let on_export_json = {
        let editor = editor.clone();
        Callback::from(move |_| {
            let file = ExcalidrawFile::new(editor.borrow().elements.clone());
            if let Ok(json) = file.to_json_pretty() {
                download_file("drawing.excalidraw", &json, "application/json");
            }
        })
    };

    let on_export_svg = {
        let editor = editor.clone();
        Callback::from(move |_| {
            let file = ExcalidrawFile::new(editor.borrow().elements.clone());
            let svg = to_svg(&file);
            download_file("drawing.svg", &svg, "image/svg+xml");
        })
    };

    let on_export_png = {
        let canvas_ref = canvas_ref.clone();
        Callback::from(move |_| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                if let Some(data) = export_png(&canvas) {
                    download_data_url("drawing.png", &data);
                }
            }
        })
    };

    let on_undo = {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |_| {
            if editor.borrow_mut().undo() {
                bump_frame();
            }
        })
    };

    let on_redo = {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |_| {
            if editor.borrow_mut().redo() {
                bump_frame();
            }
        })
    };

    html! {
        <div class="app">
            <header class="toolbar">
                <span class="brand">{"excaildraw"}</span>
                <div class="tool-group">
                    <button class={classes!("tool-btn", (*tool == Tool::Select).then_some("active"))}
                        onclick={set_tool(Tool::Select)} title="Select">{"↖"}</button>
                    <button class={classes!("tool-btn", (*tool == Tool::Rectangle).then_some("active"))}
                        onclick={set_tool(Tool::Rectangle)}>{"▭"}</button>
                    <button class={classes!("tool-btn", (*tool == Tool::Ellipse).then_some("active"))}
                        onclick={set_tool(Tool::Ellipse)}>{"○"}</button>
                    <button class={classes!("tool-btn", (*tool == Tool::Line).then_some("active"))}
                        onclick={set_tool(Tool::Line)}>{"／"}</button>
                    <button class={classes!("tool-btn", (*tool == Tool::Arrow).then_some("active"))}
                        onclick={set_tool(Tool::Arrow)}>{"→"}</button>
                    <button class={classes!("tool-btn", (*tool == Tool::Freedraw).then_some("active"))}
                        onclick={set_tool(Tool::Freedraw)}>{"✎"}</button>
                    <button class={classes!("tool-btn", (*tool == Tool::Text).then_some("active"))}
                        onclick={set_tool(Tool::Text)}>{"T"}</button>
                </div>
                <div class="tool-group">
                    <button class="tool-btn" onclick={on_undo}>{"Undo"}</button>
                    <button class="tool-btn" onclick={on_redo}>{"Redo"}</button>
                </div>
                <div class="tool-group">
                    <button class="tool-btn" onclick={on_import}>{"Import"}</button>
                    <button class="tool-btn" onclick={on_export_json}>{"JSON"}</button>
                    <button class="tool-btn" onclick={on_export_svg}>{"SVG"}</button>
                    <button class="tool-btn" onclick={on_export_png}>{"PNG"}</button>
                </div>
                <button class="tool-btn primary" onclick={on_create_room}>{"New Room"}</button>
                <span class="status">{(*status).clone()}</span>
                <span class="hint">{"Scroll zoom · Shift+pan · Del delete · Ctrl+Z undo"}</span>
            </header>
            <input type="file" accept=".excalidraw,.json" class="hidden-input" ref={file_input_ref} onchange={on_file_change} />
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
