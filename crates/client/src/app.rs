use crate::canvas::render_scene;
use crate::collab::{
    create_room, download_data_url, download_file, export_png, fetch_auth_token, persist_scene,
    set_auth_token, set_collab, set_room_cipher, throttle_cursor, with_collab, CollabHandle,
};
use crate::crypto::RoomCipher;
use crate::editor::{Editor, Tool};
use crate::viewport::Viewport;
use excaildraw_core::{Element, ElementType, ExcalidrawFile, to_svg};
use gloo::events::EventListener;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement, KeyboardEvent, MouseEvent, WheelEvent};
use yew::prelude::*;

fn sync_and_persist(editor: &Rc<RefCell<Editor>>) {
    let ed = editor.borrow();
    let mut file = ExcalidrawFile::new(ed.elements.clone());
    file.files = ed.files.clone();
    if let Ok(json) = file.to_json_pretty() {
        persist_scene(json);
    }
    with_collab(|h| h.send_update(&ed.elements));
}

#[function_component(App)]
pub fn app() -> Html {
    let canvas_ref = use_node_ref();
    let file_input_ref = use_node_ref();
    let image_input_ref = use_node_ref();
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
    let room_password = use_state(String::new);
    let layers_open = use_state(|| true);

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
                    let selected: Vec<String> = ed.selected.iter().cloned().collect();
                    render_scene(
                        &ctx,
                        &viewport.borrow(),
                        width,
                        height,
                        &ed.elements,
                        &ed.files,
                        prev.as_ref(),
                        &selected,
                        &collab,
                    );
                }
            }
            || ()
        });
    }

    {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(json) = crate::storage::load_scene().await {
                    if let Ok(parsed) = ExcalidrawFile::from_json(&json) {
                        editor.borrow_mut().load_scene(parsed.elements, parsed.files);
                        bump_frame();
                    }
                }
            });
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(token) = fetch_auth_token("guest").await {
                    set_auth_token(Some(token));
                }
            });
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
        let image_input_ref = image_input_ref.clone();
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
                    editor.borrow_mut().start_drag(wx, wy, e.shift_key() || e.meta_key());
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
                            sync_and_persist(&editor);
                            bump_frame();
                        }
                    }
                }
                Tool::Image => {
                    *drawing.borrow_mut() = Some((wx, wy));
                    if let Some(input) = image_input_ref.cast::<HtmlInputElement>() {
                        input.click();
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
                sync_and_persist(&editor);
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
                    sync_and_persist(&editor);
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
                            editor.borrow_mut().load_scene(parsed.elements, parsed.files);
                            bump();
                        }
                    }
                });
            }
        })
    };

    let on_image_change = {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        let drawing = drawing.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(file) = input.files().and_then(|f| f.get(0)) {
                let editor = editor.clone();
                let bump = bump_frame.clone();
                let pos = drawing.borrow().unwrap_or((100.0, 100.0));
                drawing.borrow_mut().take();
                wasm_bindgen_futures::spawn_local(async move {
                    let blob = gloo_file::Blob::from(file);
                    if let Ok(data_url) = gloo_file::futures::read_as_data_url(&blob).await {
                        let file_id = uuid::Uuid::new_v4().to_string();
                        let meta = serde_json::json!({
                            "mimeType": "image/png",
                            "id": file_id,
                            "dataURL": data_url,
                            "created": js_sys::Date::now() as u64
                        });
                        let mut el = Element::new(ElementType::Image, pos.0, pos.1, 240.0, 180.0);
                        el.file_id = Some(file_id.clone());
                        el.bump_version();
                        editor.borrow_mut().add_file(file_id, meta);
                        editor.borrow_mut().push_element(el);
                        sync_and_persist(&editor);
                        bump();
                    }
                });
            }
        })
    };

    let on_password_change = {
        let room_password = room_password.clone();
        let room_id = room_id.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            room_password.set(input.value());
            if !room_id.is_empty() && !input.value().is_empty() {
                set_room_cipher(Some(RoomCipher::from_password(&input.value(), &room_id)));
            } else {
                set_room_cipher(None);
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

    let layer_items = editor.borrow().layer_list();
    let selected_layers = editor.borrow().selected.clone();

    let on_layer_select = {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        move |id: String| {
            let editor = editor.clone();
            let bump = bump_frame.clone();
            Callback::from(move |_| {
                editor.borrow_mut().selected.clear();
                editor.borrow_mut().selected.insert(id.clone());
                bump();
            })
        }
    };

    let on_layer_up = {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        move |id: String| {
            let editor = editor.clone();
            let bump = bump_frame.clone();
            Callback::from(move |_| {
                editor.borrow_mut().move_layer(&id, -1);
                sync_and_persist(&editor);
                bump();
            })
        }
    };

    let on_layer_down = {
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        move |id: String| {
            let editor = editor.clone();
            let bump = bump_frame.clone();
            Callback::from(move |_| {
                editor.borrow_mut().move_layer(&id, 1);
                sync_and_persist(&editor);
                bump();
            })
        }
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
                    <button class={classes!("tool-btn", (*tool == Tool::Image).then_some("active"))}
                        onclick={set_tool(Tool::Image)}>{"🖼"}</button>
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
                <input type="password" class="room-pass" placeholder="E2E password" oninput={on_password_change} />
                <button class="tool-btn primary" onclick={on_create_room}>{"New Room"}</button>
                <span class="status">{(*status).clone()}</span>
                <span class="hint">{"Shift+click multi-select · Meta+click add"}</span>
            </header>
            <div class="main">
                if *layers_open {
                    <aside class="layers">
                        <div class="layers-head">{"Layers"}</div>
                        { for layer_items.iter().rev().map(|(id, label)| {
                            let id_up = id.clone();
                            let id_down = id.clone();
                            let id_sel = id.clone();
                            html! {
                                <div class={classes!("layer-row", selected_layers.contains(id).then_some("active"))}>
                                    <button class="layer-name" onclick={on_layer_select(id_sel)}>{label}</button>
                                    <button class="layer-btn" onclick={on_layer_up(id_up)}>{"↑"}</button>
                                    <button class="layer-btn" onclick={on_layer_down(id_down)}>{"↓"}</button>
                                </div>
                            }
                        }) }
                    </aside>
                }
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
            <input type="file" accept=".excalidraw,.json" class="hidden-input" ref={file_input_ref} onchange={on_file_change} />
            <input type="file" accept="image/*" class="hidden-input" ref={image_input_ref} onchange={on_image_change} />
        </div>
    }
}
