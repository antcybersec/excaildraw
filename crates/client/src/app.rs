use crate::collab::{
    create_room, download_data_url, download_file, export_png, fetch_auth_token, persist_scene,
    set_auth_token, set_collab, set_room_cipher, throttle_cursor, with_collab, CollabHandle,
};
use crate::crypto::RoomCipher;
use crate::editor::{element_valid, Editor, Tool};
use crate::icons::tool_icon;
use crate::render_loop;
use crate::theme::CanvasTheme;
use crate::viewport::Viewport;
use excaildraw_core::{Element, ElementType, ExcalidrawFile, to_svg};
use gloo::events::EventListener;
use gloo_events::EventListenerOptions;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{
    Element as DomElement, HtmlCanvasElement, HtmlInputElement, HtmlTextAreaElement,
    InputEvent, KeyboardEvent, MouseEvent, PointerEvent, WheelEvent, FocusEvent,
};
use yew::prelude::*;

const TEXT_FONT_SIZE: f64 = 20.0;

#[derive(Clone, PartialEq)]
struct TextEdit {
    world_x: f64,
    world_y: f64,
}

fn sync_and_persist(editor: &Rc<RefCell<Editor>>) {
    let ed = editor.borrow();
    let mut file = ExcalidrawFile::new(ed.elements.clone());
    file.files = ed.files.clone();
    if let Ok(json) = file.to_json_pretty() {
        persist_scene(json);
    }
    with_collab(|h| h.send_update(&ed.elements));
}

fn is_editable_target(e: &KeyboardEvent) -> bool {
    e.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
        .is_some_and(|el| {
            let tag = el.tag_name();
            tag.eq_ignore_ascii_case("input") || tag.eq_ignore_ascii_case("textarea")
        })
}

fn click_attr(event: &web_sys::Event, attr: &str) -> Option<String> {
    let mut node = event
        .target()
        .and_then(|t| t.dyn_into::<DomElement>().ok());
    while let Some(el) = node {
        if let Some(val) = el.get_attribute(attr) {
            if !val.is_empty() {
                return Some(val);
            }
        }
        node = el.parent_element();
    }
    None
}

fn activate_tool(
    t: Tool,
    tool: &UseStateHandle<Tool>,
    text_edit: &UseStateHandle<Option<TextEdit>>,
    drawing: &Rc<RefCell<Option<(f64, f64)>>>,
    preview: &Rc<RefCell<Option<Element>>>,
    panning: &Rc<RefCell<Option<(f64, f64)>>>,
) {
    tool.set(t);
    text_edit.set(None);
    drawing.borrow_mut().take();
    preview.borrow_mut().take();
    *panning.borrow_mut() = None;
}

fn capture_canvas(canvas: &HtmlCanvasElement, pointer_id: i32) {
    let _ = canvas.set_pointer_capture(pointer_id);
}

fn release_canvas(canvas: &HtmlCanvasElement, pointer_id: i32) {
    let _ = canvas.release_pointer_capture(pointer_id);
}

fn finish_text_edit(
    editor: &Rc<RefCell<Editor>>,
    edit: TextEdit,
    text: String,
    stroke_color: &str,
) {
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }
    let width = (text.chars().count() as f64 * TEXT_FONT_SIZE * 0.55).max(24.0);
    let height = TEXT_FONT_SIZE * 1.35;
    let mut el = Element::new(ElementType::Text, edit.world_x, edit.world_y, width, height);
    el.text = Some(text);
    el.font_size = Some(TEXT_FONT_SIZE);
    el.stroke_color = stroke_color.to_string();
    el.bump_version();
    editor.borrow_mut().push_element(el);
    sync_and_persist(editor);
}

fn commit_open_text_edit(
    text_area_ref: &NodeRef,
    text_edit: &UseStateHandle<Option<TextEdit>>,
    editor: &Rc<RefCell<Editor>>,
    ignore_blur: &Rc<Cell<bool>>,
    stroke_color: &str,
) -> bool {
    let Some(edit) = (**text_edit).clone() else {
        return false;
    };
    let text = text_area_ref
        .cast::<HtmlTextAreaElement>()
        .map(|ta| ta.value())
        .unwrap_or_default();
    ignore_blur.set(true);
    text_edit.set(None);
    finish_text_edit(editor, edit, text, stroke_color);
    let ignore_blur = ignore_blur.clone();
    gloo_timers::callback::Timeout::new(0, move || ignore_blur.set(false)).forget();
    true
}

fn append_freedraw_point(points: &mut Vec<f64>, wx: f64, wy: f64, min_dist: f64) {
    if points.len() >= 2 {
        let lx = points[points.len() - 2];
        let ly = points[points.len() - 1];
        if (wx - lx).hypot(wy - ly) < min_dist {
            return;
        }
    }
    points.extend([wx, wy]);
}

fn shape_bounds(
    sx0: f64,
    sy0: f64,
    wx: f64,
    wy: f64,
    constrain: bool,
) -> (f64, f64, f64, f64) {
    let mut w = (wx - sx0).abs().max(1.0);
    let mut h = (wy - sy0).abs().max(1.0);
    if constrain {
        let side = w.max(h);
        w = side;
        h = side;
    }
    let x = if wx >= sx0 { sx0 } else { sx0 - w };
    let y = if wy >= sy0 { sy0 } else { sy0 - h };
    (x, y, w, h)
}

fn line_points(
    sx0: f64,
    sy0: f64,
    wx: f64,
    wy: f64,
    constrain: bool,
) -> (f64, f64, f64, f64) {
    if !constrain {
        return (sx0, sy0, wx, wy);
    }
    let dx = wx - sx0;
    let dy = wy - sy0;
    let angle = (dy.atan2(dx) / (std::f64::consts::PI / 4.0)).round() * (std::f64::consts::PI / 4.0);
    let len = (dx * dx + dy * dy).sqrt();
    (sx0, sy0, sx0 + len * angle.cos(), sy0 + len * angle.sin())
}

#[function_component(App)]
pub fn app() -> Html {
    let canvas_ref = use_node_ref();
    let app_ref = use_node_ref();
    let text_area_ref = use_node_ref();
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
    let space_held = use_state(|| false);
    let text_edit = use_state(|| None::<TextEdit>);
    let ignore_text_blur = use_state(|| Rc::new(Cell::new(false)));
    let theme = use_mut_ref(CanvasTheme::default);
    let erasing = use_mut_ref(|| false);
    let eraser_modified = use_mut_ref(|| false);
    let eraser_history_saved = use_mut_ref(|| false);
    let layers_snapshot = use_state(|| (Vec::<(String, String)>::new(), std::collections::HashSet::<String>::new()));

    let bump_frame = {
        let frame = frame.clone();
        let editor = editor.clone();
        let layers_snapshot = layers_snapshot.clone();
        Rc::new(move || {
            frame.set(*frame + 1);
            if let Ok(ed) = editor.try_borrow() {
                layers_snapshot.set((ed.layer_list(), ed.selected.clone()));
            }
        })
    };

    {
        let theme = theme.clone();
        use_effect_with((), move |_| {
            crate::theme::apply_document_class(theme.borrow().dark);
            || ()
        });
    }

    {
        let canvas_ref = canvas_ref.clone();
        let viewport = viewport.clone();
        let editor = editor.clone();
        let preview = preview.clone();
        let collaborators = collaborators.clone();
        let theme = theme.clone();
        use_effect_with((), move |_| {
            render_loop::start(canvas_ref, viewport, editor, preview, collaborators, theme)
        });
    }

    {
        let text_area_ref = text_area_ref.clone();
        let text_edit = text_edit.clone();
        use_effect_with(text_edit.clone(), move |edit| {
            if edit.is_some() {
                let text_area_ref = text_area_ref.clone();
                gloo_timers::callback::Timeout::new(0, move || {
                    if let Some(ta) = text_area_ref.cast::<HtmlTextAreaElement>() {
                        ta.set_value("");
                        let _ = ta.focus();
                        let len = ta.value().len() as u32;
                        let _ = ta.set_selection_range(len, len);
                    }
                })
                .forget();
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
        let tool = tool.clone();
        let space_held = space_held.clone();
        let text_edit = text_edit.clone();
        let drawing = drawing.clone();
        let preview = preview.clone();
        let panning = panning.clone();
        use_effect_with((), move |_| {
            let window = web_sys::window().unwrap();
            let listener_down = EventListener::new(&window, "keydown", {
                let editor = editor.clone();
                let bump_frame = bump_frame.clone();
                let tool = tool.clone();
                let space_held = space_held.clone();
                let text_edit = text_edit.clone();
                let drawing = drawing.clone();
                let preview = preview.clone();
                let panning = panning.clone();
                move |e| {
                    let e = e.dyn_ref::<KeyboardEvent>().unwrap();
                    if is_editable_target(e) {
                        return;
                    }
                    let key = e.key();
                    if key == " " {
                        e.prevent_default();
                        space_held.set(true);
                        return;
                    }
                    if let Ok(n) = key.parse::<u8>() {
                        if let Some(t) = Tool::from_number(n) {
                            activate_tool(t, &tool, &text_edit, &drawing, &preview, &panning);
                            return;
                        }
                    }
                    if let Some(t) = Tool::from_key(&key) {
                        activate_tool(t, &tool, &text_edit, &drawing, &preview, &panning);
                        return;
                    }
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
                        sync_and_persist(&editor);
                        bump_frame();
                    }
                }
            });
            let listener_up = EventListener::new(&window, "keyup", {
                let space_held = space_held.clone();
                move |e| {
                    let e = e.dyn_ref::<KeyboardEvent>().unwrap();
                    if e.key() == " " {
                        space_held.set(false);
                    }
                }
            });
            move || {
                drop(listener_down);
                drop(listener_up);
            }
        });
    }

    {
        let canvas_ref = canvas_ref.clone();
        let viewport = viewport.clone();
        use_effect_with((), move |_| {
            let listener = canvas_ref.cast::<HtmlCanvasElement>().map(|canvas| {
                let viewport = viewport.clone();
                let canvas_ref = canvas_ref.clone();
                EventListener::new_with_options(
                    &canvas,
                    "wheel",
                    EventListenerOptions::enable_prevent_default(),
                    move |e| {
                        let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() else {
                            return;
                        };
                        let e = e.dyn_ref::<WheelEvent>().unwrap();
                        e.prevent_default();
                        let rect = canvas.get_bounding_client_rect();
                        let sx = e.client_x() as f64 - rect.left();
                        let sy = e.client_y() as f64 - rect.top();
                        viewport.borrow_mut().zoom_at(sx, sy, e.delta_y());
                    },
                )
            });
            move || drop(listener)
        });
    }

    let on_pointer_down = {
        let viewport = viewport.clone();
        let canvas_ref = canvas_ref.clone();
        let text_area_ref = text_area_ref.clone();
        let tool = tool.clone();
        let drawing = drawing.clone();
        let preview = preview.clone();
        let panning = panning.clone();
        let freedraw_points = freedraw_points.clone();
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        let username = username.clone();
        let image_input_ref = image_input_ref.clone();
        let space_held = space_held.clone();
        let text_edit = text_edit.clone();
        let ignore_text_blur = ignore_text_blur.clone();
        let theme = theme.clone();
        let erasing = erasing.clone();
        let eraser_modified = eraser_modified.clone();
        let eraser_history_saved = eraser_history_saved.clone();
        Callback::from(move |e: PointerEvent| {
            let canvas_el = canvas_ref.cast::<HtmlCanvasElement>();

            let Some((wx, wy, sx, sy)) = (|| {
                let rect = canvas_ref.cast::<HtmlCanvasElement>()?.get_bounding_client_rect();
                let sx = e.client_x() as f64 - rect.left();
                let sy = e.client_y() as f64 - rect.top();
                let (wx, wy) = viewport.borrow().screen_to_world(sx, sy);
                Some((wx, wy, sx, sy))
            })() else {
                return;
            };

            if let Some(canvas) = canvas_el.as_ref() {
                if *tool != Tool::Text {
                    let _ = canvas.focus();
                }
            }

            let pan_mode = e.button() == 1 || (e.button() == 0 && *space_held);
            if pan_mode {
                *panning.borrow_mut() = Some((sx, sy));
                if let Some(canvas) = canvas_el.as_ref() {
                    capture_canvas(canvas, e.pointer_id());
                }
                return;
            }

            if e.button() != 0 {
                return;
            }

            match *tool {
                Tool::Select => {
                    editor.borrow_mut().start_drag(wx, wy, e.shift_key() || e.meta_key());
                    if editor.borrow().dragging.is_some() {
                        if let Some(canvas) = canvas_el.as_ref() {
                            capture_canvas(canvas, e.pointer_id());
                        }
                    }
                    bump_frame();
                }
                Tool::Rectangle => {
                    if let Some(canvas) = canvas_el.as_ref() {
                        capture_canvas(canvas, e.pointer_id());
                    }
                    *drawing.borrow_mut() = Some((wx, wy));
                    let mut el = Element::new(ElementType::Rectangle, wx, wy, 0.0, 0.0);
                    el.stroke_color = theme.borrow().default_stroke().to_string();
                    *preview.borrow_mut() = Some(el);
                }
                Tool::Ellipse => {
                    if let Some(canvas) = canvas_el.as_ref() {
                        capture_canvas(canvas, e.pointer_id());
                    }
                    *drawing.borrow_mut() = Some((wx, wy));
                    let mut el = Element::new(ElementType::Ellipse, wx, wy, 0.0, 0.0);
                    el.stroke_color = theme.borrow().default_stroke().to_string();
                    *preview.borrow_mut() = Some(el);
                }
                Tool::Line | Tool::Arrow => {
                    if let Some(canvas) = canvas_el.as_ref() {
                        capture_canvas(canvas, e.pointer_id());
                    }
                    *drawing.borrow_mut() = Some((wx, wy));
                    let t = if *tool == Tool::Arrow {
                        ElementType::Arrow
                    } else {
                        ElementType::Line
                    };
                    let mut el = Element::new(t, wx, wy, 0.0, 0.0);
                    el.stroke_color = theme.borrow().default_stroke().to_string();
                    el.points = Some(vec![wx, wy, wx, wy]);
                    *preview.borrow_mut() = Some(el);
                }
                Tool::Freedraw => {
                    if let Some(canvas) = canvas_el.as_ref() {
                        capture_canvas(canvas, e.pointer_id());
                    }
                    freedraw_points.borrow_mut().clear();
                    freedraw_points.borrow_mut().extend([wx, wy]);
                    let mut el = Element::new(ElementType::Freedraw, wx, wy, 0.0, 0.0);
                    el.stroke_color = theme.borrow().default_stroke().to_string();
                    el.points = Some(freedraw_points.borrow().clone());
                    *preview.borrow_mut() = Some(el);
                    *drawing.borrow_mut() = Some((wx, wy));
                }
                Tool::Text => {
                    let stroke = theme.borrow().default_stroke();
                    commit_open_text_edit(
                        &text_area_ref,
                        &text_edit,
                        &editor,
                        &ignore_text_blur,
                        stroke,
                    );
                    text_edit.set(Some(TextEdit {
                        world_x: wx,
                        world_y: wy,
                    }));
                    bump_frame();
                }
                Tool::Eraser => {
                    if let Some(canvas) = canvas_el.as_ref() {
                        capture_canvas(canvas, e.pointer_id());
                    }
                    *erasing.borrow_mut() = true;
                    *eraser_modified.borrow_mut() = false;
                    *eraser_history_saved.borrow_mut() = false;
                    let mut saved = false;
                    if editor
                        .borrow_mut()
                        .erase_at_stroke(wx, wy, &mut saved)
                    {
                        *eraser_history_saved.borrow_mut() = saved;
                        *eraser_modified.borrow_mut() = true;
                        bump_frame();
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
        })
    };

    let on_pointer_move = {
        let viewport = viewport.clone();
        let canvas_ref = canvas_ref.clone();
        let drawing = drawing.clone();
        let preview = preview.clone();
        let panning = panning.clone();
        let freedraw_points = freedraw_points.clone();
        let tool = tool.clone();
        let editor = editor.clone();
        let username = username.clone();
        let erasing = erasing.clone();
        let eraser_modified = eraser_modified.clone();
        let eraser_history_saved = eraser_history_saved.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |e: PointerEvent| {
            let rect = canvas_ref.cast::<HtmlCanvasElement>().map(|c| c.get_bounding_client_rect());
            let Some(rect) = rect else { return };
            let sx = e.client_x() as f64 - rect.left();
            let sy = e.client_y() as f64 - rect.top();

            if let Some((px, py)) = *panning.borrow() {
                viewport.borrow_mut().pan_immediate(sx - px, sy - py);
                *panning.borrow_mut() = Some((sx, sy));
                return;
            }

            let (wx, wy) = viewport.borrow().screen_to_world(sx, sy);

            if editor.borrow().dragging.is_some() {
                editor.borrow_mut().drag_to(wx, wy);
                return;
            }

            if *tool == Tool::Eraser && *erasing.borrow() {
                let mut saved = *eraser_history_saved.borrow();
                if editor
                    .borrow_mut()
                    .erase_at_stroke(wx, wy, &mut saved)
                {
                    *eraser_history_saved.borrow_mut() = saved;
                    *eraser_modified.borrow_mut() = true;
                    bump_frame();
                }
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
                            let (x, y, w, h) =
                                shape_bounds(sx0, sy0, wx, wy, e.shift_key());
                            p.x = x;
                            p.y = y;
                            p.width = w;
                            p.height = h;
                        }
                    }
                }
                Tool::Line | Tool::Arrow => {
                    if let Some((sx0, sy0)) = *drawing.borrow() {
                        if let Some(p) = preview.borrow_mut().as_mut() {
                            let (x1, y1, x2, y2) =
                                line_points(sx0, sy0, wx, wy, e.shift_key());
                            p.points = Some(vec![x1, y1, x2, y2]);
                        }
                    }
                }
                Tool::Freedraw => {
                    let min_dist = (1.5 / viewport.borrow().zoom).max(0.5);
                    append_freedraw_point(&mut freedraw_points.borrow_mut(), wx, wy, min_dist);
                    if let Some(p) = preview.borrow_mut().as_mut() {
                        p.points = Some(freedraw_points.borrow().clone());
                    }
                }
                _ => {}
            }
            throttle_cursor(&username, wx, wy);
        })
    };

    let on_pointer_up = {
        let canvas_ref = canvas_ref.clone();
        let drawing = drawing.clone();
        let preview = preview.clone();
        let editor = editor.clone();
        let panning = panning.clone();
        let bump_frame = bump_frame.clone();
        let erasing = erasing.clone();
        let eraser_modified = eraser_modified.clone();
        Callback::from(move |e: PointerEvent| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                release_canvas(&canvas, e.pointer_id());
            }

            if panning.borrow_mut().take().is_some() {
                return;
            }
            if *erasing.borrow() {
                *erasing.borrow_mut() = false;
                if *eraser_modified.borrow() {
                    sync_and_persist(&editor);
                }
                *eraser_modified.borrow_mut() = false;
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
                if element_valid(&el) {
                    el.bump_version();
                    editor.borrow_mut().push_element(el);
                    sync_and_persist(&editor);
                    bump_frame();
                }
            }
            drawing.borrow_mut().take();
        })
    };

    {
        let tool = tool.clone();
        let text_edit = text_edit.clone();
        let drawing = drawing.clone();
        let preview = preview.clone();
        let panning = panning.clone();
        let editor = editor.clone();
        let bump_frame = bump_frame.clone();
        let file_input_ref = file_input_ref.clone();
        let canvas_ref = canvas_ref.clone();
        let room_id = room_id.clone();
        let status = status.clone();
        let collaborators = collaborators.clone();
        let username = username.clone();
        let theme = theme.clone();
        use_effect_with((), move |_| {
            let document = web_sys::window()
                .and_then(|w| w.document())
                .expect("document");
            let listener = EventListener::new(&document, "click", move |e| {
                let e = e.dyn_ref::<web_sys::Event>().unwrap();
                if let Some(tool_id) = click_attr(e, "data-tool") {
                    e.prevent_default();
                    e.stop_propagation();
                    if let Ok(n) = tool_id.parse::<u8>() {
                        if let Some(t) = Tool::from_number(n) {
                            activate_tool(t, &tool, &text_edit, &drawing, &preview, &panning);
                        }
                    }
                    return;
                }
                let Some(action) = click_attr(e, "data-action") else {
                    return;
                };
                e.prevent_default();
                e.stop_propagation();
                match action.as_str() {
                    "undo" => {
                        if editor.borrow_mut().undo() {
                            bump_frame();
                        }
                    }
                    "redo" => {
                        if editor.borrow_mut().redo() {
                            bump_frame();
                        }
                    }
                    "import" => {
                        if let Some(input) = file_input_ref.cast::<HtmlInputElement>() {
                            input.click();
                        }
                    }
                    "export-json" => {
                        let file = ExcalidrawFile::new(editor.borrow().elements.clone());
                        if let Ok(json) = file.to_json_pretty() {
                            download_file("drawing.excalidraw", &json, "application/json");
                        }
                    }
                    "export-svg" => {
                        let file = ExcalidrawFile::new(editor.borrow().elements.clone());
                        download_file("drawing.svg", &to_svg(&file), "image/svg+xml");
                    }
                    "export-png" => {
                        if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                            if let Some(data) = export_png(&canvas) {
                                download_data_url("drawing.png", &data);
                            }
                        }
                    }
                    "toggle-theme" => {
                        theme.borrow_mut().toggle();
                        bump_frame();
                    }
                    "collab" => {
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
                    }
                    _ => {}
                }
            });
            move || drop(listener)
        });
    }

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

    let (layer_items, selected_layers) = (*layers_snapshot).clone();

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

    let on_text_keydown = {
        let editor = editor.clone();
        let text_edit = text_edit.clone();
        let text_area_ref = text_area_ref.clone();
        let ignore_text_blur = ignore_text_blur.clone();
        let theme = theme.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" && !e.shift_key() {
                e.prevent_default();
                e.stop_propagation();
                let stroke = theme.borrow().default_stroke();
                if commit_open_text_edit(
                    &text_area_ref,
                    &text_edit,
                    &editor,
                    &ignore_text_blur,
                    stroke,
                ) {
                    bump_frame();
                }
            } else if e.key() == "Escape" {
                e.prevent_default();
                text_edit.set(None);
            }
        })
    };

    let on_text_blur = {
        let editor = editor.clone();
        let text_edit = text_edit.clone();
        let text_area_ref = text_area_ref.clone();
        let ignore_text_blur = ignore_text_blur.clone();
        let theme = theme.clone();
        let bump_frame = bump_frame.clone();
        Callback::from(move |_e: FocusEvent| {
            if text_edit.is_none() || ignore_text_blur.get() {
                return;
            }
            let stroke = theme.borrow().default_stroke();
            if commit_open_text_edit(
                &text_area_ref,
                &text_edit,
                &editor,
                &ignore_text_blur,
                stroke,
            ) {
                bump_frame();
            }
        })
    };

    let on_text_input = {
        let text_area_ref = text_area_ref.clone();
        let viewport = viewport.clone();
        Callback::from(move |_e: InputEvent| {
            if let Some(ta) = text_area_ref.cast::<HtmlTextAreaElement>() {
                let zoom = viewport.try_borrow().map(|v| v.zoom).unwrap_or(1.0);
                let font_size = TEXT_FONT_SIZE * zoom;
                let w = ta.scroll_width().max(8) + 4;
                let h = font_size.ceil() as i32;
                let _ = ta.style().set_property("width", &format!("{w}px"));
                let _ = ta.style().set_property("height", &format!("{h}px"));
            }
        })
    };

    let on_text_pointerdown = Callback::from(|e: PointerEvent| {
        e.stop_propagation();
    });

    let canvas_class = classes!(
        "canvas",
        (*space_held).then_some("panning"),
        match *tool {
            Tool::Select => "tool-select",
            Tool::Text => "tool-text",
            Tool::Eraser => "tool-eraser",
            _ => "tool-draw",
        }
    );

    let text_zoom = viewport.try_borrow().map(|v| v.zoom).unwrap_or(1.0);
    let text_screen_pos = text_edit.as_ref().and_then(|edit| {
        viewport
            .try_borrow()
            .ok()
            .map(|v| v.world_to_screen(edit.world_x, edit.world_y))
    });
    let theme_snapshot = theme.borrow().clone();
    let text_stroke = theme_snapshot.default_stroke();
    let dark_canvas = theme_snapshot.dark;

    html! {
        <div class="app" ref={app_ref}>
            <header class="top-bar">
                <div class="brand">{"rust"}<span>{"Canvas"}</span></div>
                <div class="chip-group">
                    <button type="button" class="chip" data-action="undo">{"Undo"}</button>
                    <button type="button" class="chip" data-action="redo">{"Redo"}</button>
                </div>
                <div class="chip-group">
                    <button type="button" class="chip" data-action="import">{"Import"}</button>
                    <button type="button" class="chip" data-action="export-json">{"JSON"}</button>
                    <button type="button" class="chip" data-action="export-svg">{"SVG"}</button>
                    <button type="button" class="chip" data-action="export-png">{"PNG"}</button>
                    <button type="button" class="chip" data-action="toggle-theme"
                        title="Toggle dark canvas">
                        { if dark_canvas { "Light" } else { "Dark" } }
                    </button>
                </div>
                <input type="password" class="room-pass" placeholder="Room password" oninput={on_password_change} />
                <button type="button" class="chip primary" data-action="collab">{"Live collab"}</button>
                <span class="status">{(*status).clone()}</span>
                <span class="spacer"></span>
                <span class="hint">{"1–9 tools · E eraser · Shift constrain · Space pan"}</span>
            </header>
            <div class="workspace">
                <aside class="tool-rail">
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Select).then_some("active"))}
                        data-tool="1" title="Selection (1)">
                        { tool_icon(Tool::Select) }<span class="key-hint">{"1"}</span>
                    </button>
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Rectangle).then_some("active"))}
                        data-tool="2" title="Rectangle (2)">
                        { tool_icon(Tool::Rectangle) }<span class="key-hint">{"2"}</span>
                    </button>
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Ellipse).then_some("active"))}
                        data-tool="3" title="Ellipse (3)">
                        { tool_icon(Tool::Ellipse) }<span class="key-hint">{"3"}</span>
                    </button>
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Line).then_some("active"))}
                        data-tool="4" title="Line (4)">
                        { tool_icon(Tool::Line) }<span class="key-hint">{"4"}</span>
                    </button>
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Arrow).then_some("active"))}
                        data-tool="5" title="Arrow (5)">
                        { tool_icon(Tool::Arrow) }<span class="key-hint">{"5"}</span>
                    </button>
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Freedraw).then_some("active"))}
                        data-tool="6" title="Draw (6)">
                        { tool_icon(Tool::Freedraw) }<span class="key-hint">{"6"}</span>
                    </button>
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Text).then_some("active"))}
                        data-tool="7" title="Text (7)">
                        { tool_icon(Tool::Text) }<span class="key-hint">{"7"}</span>
                    </button>
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Image).then_some("active"))}
                        data-tool="8" title="Image (8)">
                        { tool_icon(Tool::Image) }<span class="key-hint">{"8"}</span>
                    </button>
                    <button type="button" class={classes!("tool-btn", (*tool == Tool::Eraser).then_some("active"))}
                        data-tool="9" title="Eraser (9 / E)">
                        { tool_icon(Tool::Eraser) }<span class="key-hint">{"9"}</span>
                    </button>
                </aside>
                <div class="canvas-area">
                    <div class="canvas-wrap">
                        <canvas
                            ref={canvas_ref}
                            class={canvas_class}
                            tabindex="0"
                            onpointerdown={on_pointer_down}
                            onpointermove={on_pointer_move}
                            onpointerup={on_pointer_up.clone()}
                            onpointercancel={on_pointer_up}
                            oncontextmenu={Callback::from(|e: MouseEvent| e.prevent_default())}
                        />
                        if text_edit.is_some() {
                            <textarea
                                ref={text_area_ref}
                                class="text-editor"
                                wrap="off"
                                rows="1"
                                style={format!(
                                    "left:{}px;top:{}px;font-size:{}px;line-height:1;color:{};caret-color:#6965db;",
                                    text_screen_pos.map(|p| p.0).unwrap_or(0.0),
                                    text_screen_pos.map(|p| p.1).unwrap_or(0.0),
                                    TEXT_FONT_SIZE * text_zoom,
                                    text_stroke,
                                )}
                                onpointerdown={on_text_pointerdown}
                                oninput={on_text_input}
                                onkeydown={on_text_keydown}
                                onblur={on_text_blur}
                            />
                        }
                    </div>
                </div>
                if *layers_open {
                    <aside class="layers">
                        <div class="layers-head">{"Layers"}</div>
                        { for layer_items.iter().rev().map(|(id, label)| {
                            let id_up = id.clone();
                            let id_down = id.clone();
                            let id_sel = id.clone();
                            html! {
                                <div class={classes!("layer-row", selected_layers.contains(id).then_some("active"))}>
                                    <button type="button" class="layer-name" onclick={on_layer_select(id_sel)}>{label}</button>
                                    <button type="button" class="layer-btn" onclick={on_layer_up(id_up)}>{"↑"}</button>
                                    <button type="button" class="layer-btn" onclick={on_layer_down(id_down)}>{"↓"}</button>
                                </div>
                            }
                        }) }
                    </aside>
                }
            </div>
            <input type="file" accept=".excalidraw,.json" class="hidden-input" ref={file_input_ref} onchange={on_file_change} />
            <input type="file" accept="image/*" class="hidden-input" ref={image_input_ref} onchange={on_image_change} />
        </div>
    }
}
