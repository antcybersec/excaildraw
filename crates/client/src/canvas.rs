use excaildraw_core::{
    element_bounds, rough_ellipse, rough_polyline, rough_rectangle, Element, ElementType,
};
use serde_json::Value;
use std::collections::HashMap;
use web_sys::CanvasRenderingContext2d;

use crate::editor::Collaborators;
use crate::theme::CanvasTheme;
use crate::viewport::Viewport;

#[allow(clippy::too_many_arguments)]
pub fn render_scene(
    ctx: &CanvasRenderingContext2d,
    viewport: &Viewport,
    width: f64,
    height: f64,
    theme: &CanvasTheme,
    elements: &[Element],
    files: &HashMap<String, Value>,
    preview: Option<&Element>,
    selected: &[String],
    collaborators: &Collaborators,
) {
    ctx.set_fill_style_str(theme.canvas_background());
    ctx.fill_rect(0.0, 0.0, width, height);

    ctx.save();
    ctx.translate(viewport.offset_x, viewport.offset_y).ok();
    ctx.scale(viewport.zoom, viewport.zoom).ok();

    ctx.set_line_cap("round");
    ctx.set_line_join("round");

    for element in elements.iter().filter(|e| !e.is_deleted) {
        draw_element(ctx, element, files, false);
        if selected.contains(&element.id) {
            draw_selection(ctx, element, theme);
        }
    }

    if let Some(preview) = preview {
        let mut crisp = preview.clone();
        crisp.roughness = 0.0;
        ctx.set_global_alpha(0.75);
        draw_element(ctx, &crisp, files, true);
        ctx.set_global_alpha(1.0);
    }

    for (user, (x, y, color)) in collaborators {
        draw_cursor(ctx, user, *x, *y, color);
    }

    ctx.restore();
}

fn draw_path(ctx: &CanvasRenderingContext2d, points: &[(f64, f64)], close: bool) {
    if points.is_empty() {
        return;
    }
    ctx.begin_path();
    ctx.move_to(points[0].0, points[0].1);
    for p in &points[1..] {
        ctx.line_to(p.0, p.1);
    }
    if close {
        ctx.close_path();
    }
    ctx.stroke();
}

fn draw_selection(ctx: &CanvasRenderingContext2d, element: &Element, theme: &CanvasTheme) {
    let (x, y, w, h) = element_bounds(element);
    ctx.set_stroke_style_str(theme.selection_color());
    ctx.set_line_width(1.0);
    ctx.set_global_alpha(1.0);
    ctx.stroke_rect(x - 4.0, y - 4.0, w + 8.0, h + 8.0);
}

fn draw_cursor(ctx: &CanvasRenderingContext2d, user: &str, x: f64, y: f64, color: &str) {
    ctx.set_fill_style_str(color);
    ctx.begin_path();
    ctx.arc(x, y, 4.0, 0.0, std::f64::consts::TAU).ok();
    ctx.fill();
    ctx.set_font("12px sans-serif");
    ctx.fill_text(user, x + 8.0, y - 8.0).ok();
}

fn draw_element(
    ctx: &CanvasRenderingContext2d,
    element: &Element,
    files: &HashMap<String, Value>,
    _is_preview: bool,
) {
    ctx.set_stroke_style_str(&element.stroke_color);
    ctx.set_fill_style_str(&element.background_color);
    ctx.set_line_width(element.stroke_width);
    ctx.set_global_alpha(element.opacity / 100.0);
    apply_stroke_style(ctx, element);

    match element.element_type {
        ElementType::Rectangle => {
            let path = rough_rectangle(
                element.seed,
                element.x,
                element.y,
                element.width,
                element.height,
                element.roughness,
                element.stroke_width,
            );
            if element.background_color != "transparent" {
                fill_path(ctx, &path);
            }
            draw_path(ctx, &path, true);
        }
        ElementType::Ellipse => {
            let cx = element.x + element.width / 2.0;
            let cy = element.y + element.height / 2.0;
            let path = rough_ellipse(
                element.seed,
                cx,
                cy,
                element.width.abs() / 2.0,
                element.height.abs() / 2.0,
                element.roughness,
                element.stroke_width,
            );
            if element.background_color != "transparent" {
                fill_path(ctx, &path);
            }
            draw_path(ctx, &path, true);
        }
        ElementType::Line => draw_line(ctx, element, false, &element.stroke_color),
        ElementType::Arrow => draw_line(ctx, element, true, &element.stroke_color),
        ElementType::Freedraw => draw_freedraw_smooth(ctx, element),
        ElementType::Text => {
            if let Some(text) = &element.text {
                let size = element.font_size.unwrap_or(20.0);
                ctx.set_fill_style_str(&element.stroke_color);
                ctx.set_font(&format!("{size}px Virgil, Segoe UI, sans-serif"));
                ctx.fill_text(text, element.x, element.y + size).ok();
            }
        }
        ElementType::Image => draw_image(ctx, element, files),
        _ => {}
    }
    ctx.set_global_alpha(1.0);
}

fn apply_stroke_style(ctx: &CanvasRenderingContext2d, element: &Element) {
    use excaildraw_core::StrokeStyle;
    match element.stroke_style {
        StrokeStyle::Dashed => ctx.set_line_dash(&js_sys::Array::of2(&10.into(), &5.into())).ok(),
        StrokeStyle::Dotted => ctx.set_line_dash(&js_sys::Array::of2(&2.into(), &4.into())).ok(),
        StrokeStyle::Solid => ctx.set_line_dash(&js_sys::Array::new()).ok(),
    };
}

fn fill_path(ctx: &CanvasRenderingContext2d, points: &[(f64, f64)]) {
    if points.len() < 3 {
        return;
    }
    ctx.begin_path();
    ctx.move_to(points[0].0, points[0].1);
    for p in &points[1..] {
        ctx.line_to(p.0, p.1);
    }
    ctx.close_path();
    ctx.fill();
}

fn draw_line(
    ctx: &CanvasRenderingContext2d,
    element: &Element,
    arrow: bool,
    stroke_color: &str,
) {
    if let Some(points) = &element.points {
        let path = rough_polyline(
            element.seed,
            points,
            element.roughness,
            element.stroke_width,
            false,
        );
        draw_path(ctx, &path, false);
        if arrow && points.len() >= 4 {
            draw_arrowhead(ctx, points[0], points[1], points[2], points[3], stroke_color);
        }
    }
}

fn draw_arrowhead(
    ctx: &CanvasRenderingContext2d,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    color: &str,
) {
    ctx.set_fill_style_str(color);
    let angle = (y2 - y1).atan2(x2 - x1);
    let size = 12.0;
    let a1 = angle + std::f64::consts::PI * 0.85;
    let a2 = angle - std::f64::consts::PI * 0.85;
    ctx.begin_path();
    ctx.move_to(x2, y2);
    ctx.line_to(x2 + size * a1.cos(), y2 + size * a1.sin());
    ctx.line_to(x2 + size * a2.cos(), y2 + size * a2.sin());
    ctx.close_path();
    ctx.fill();
}

fn draw_freedraw_smooth(ctx: &CanvasRenderingContext2d, element: &Element) {
    let Some(points) = &element.points else { return };
    if points.len() < 4 {
        return;
    }

    let pairs: Vec<(f64, f64)> = points
        .chunks(2)
        .filter_map(|c| (c.len() == 2).then_some((c[0], c[1])))
        .collect();

    if pairs.len() < 2 {
        return;
    }

    ctx.begin_path();
    ctx.move_to(pairs[0].0, pairs[0].1);

    if pairs.len() == 2 {
        ctx.line_to(pairs[1].0, pairs[1].1);
    } else {
        for i in 1..pairs.len() - 1 {
            let (cx, cy) = pairs[i];
            let (mx, my) = (
                (cx + pairs[i + 1].0) / 2.0,
                (cy + pairs[i + 1].1) / 2.0,
            );
            ctx.quadratic_curve_to(cx, cy, mx, my);
        }
        let last = pairs[pairs.len() - 1];
        ctx.line_to(last.0, last.1);
    }
    ctx.stroke();
}

fn draw_image(ctx: &CanvasRenderingContext2d, element: &Element, files: &HashMap<String, Value>) {
    let Some(file_id) = &element.file_id else { return };
    let Some(file) = files.get(file_id) else { return };
    let Some(data_url) = file.get("dataURL").and_then(|v| v.as_str()) else {
        return;
    };
    if let Ok(img) = web_sys::HtmlImageElement::new() {
        img.set_src(data_url);
        let _ = ctx.draw_image_with_html_image_element_and_dw_and_dh(
            &img,
            element.x,
            element.y,
            element.width.max(1.0),
            element.height.max(1.0),
        );
    }
}
