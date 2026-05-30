use excaildraw_core::{element_bounds, Element, ElementType};
use web_sys::CanvasRenderingContext2d;
use crate::editor::Collaborators;
use crate::viewport::Viewport;

#[allow(clippy::too_many_arguments)]
pub fn render_scene(
    ctx: &CanvasRenderingContext2d,
    viewport: &Viewport,
    width: f64,
    height: f64,
    elements: &[Element],
    preview: Option<&Element>,
    selected: Option<&str>,
    collaborators: &Collaborators,
) {
    ctx.set_fill_style_str("#f8f9fa");
    ctx.fill_rect(0.0, 0.0, width, height);

    ctx.save();
    ctx.translate(viewport.offset_x, viewport.offset_y).ok();
    ctx.scale(viewport.zoom, viewport.zoom).ok();

    draw_grid(ctx, viewport, width, height);

    for element in elements.iter().filter(|e| !e.is_deleted) {
        draw_element(ctx, element);
        if selected == Some(element.id.as_str()) {
            draw_selection(ctx, element);
        }
    }

    if let Some(preview) = preview {
        ctx.set_global_alpha(0.6);
        draw_element(ctx, preview);
        ctx.set_global_alpha(1.0);
    }

    for (user, (x, y, color)) in collaborators {
        draw_cursor(ctx, user, *x, *y, color);
    }

    ctx.restore();
}

fn draw_selection(ctx: &CanvasRenderingContext2d, element: &Element) {
    let (x, y, w, h) = element_bounds(element);
    ctx.set_stroke_style_str("#6965db");
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

fn draw_grid(ctx: &CanvasRenderingContext2d, viewport: &Viewport, width: f64, height: f64) {
    let step = 20.0;
    let (min_x, min_y) = viewport.screen_to_world(0.0, 0.0);
    let (max_x, max_y) = viewport.screen_to_world(width, height);

    let start_x = (min_x / step).floor() * step;
    let start_y = (min_y / step).floor() * step;

    ctx.set_stroke_style_str("#e9ecef");
    ctx.set_line_width(1.0 / viewport.zoom);

    let mut x = start_x;
    while x <= max_x {
        ctx.begin_path();
        ctx.move_to(x, min_y);
        ctx.line_to(x, max_y);
        ctx.stroke();
        x += step;
    }

    let mut y = start_y;
    while y <= max_y {
        ctx.begin_path();
        ctx.move_to(min_x, y);
        ctx.line_to(max_x, y);
        ctx.stroke();
        y += step;
    }
}

fn draw_element(ctx: &CanvasRenderingContext2d, element: &Element) {
    ctx.set_stroke_style_str(&element.stroke_color);
    ctx.set_fill_style_str(&element.background_color);
    ctx.set_line_width(element.stroke_width);
    ctx.set_global_alpha(element.opacity / 100.0);

    match element.element_type {
        ElementType::Rectangle => {
            if element.background_color != "transparent" {
                ctx.fill_rect(element.x, element.y, element.width, element.height);
            }
            ctx.stroke_rect(element.x, element.y, element.width, element.height);
        }
        ElementType::Ellipse => {
            let cx = element.x + element.width / 2.0;
            let cy = element.y + element.height / 2.0;
            let rx = element.width.abs() / 2.0;
            let ry = element.height.abs() / 2.0;
            ctx.begin_path();
            ctx.ellipse(cx, cy, rx, ry, 0.0, 0.0, std::f64::consts::TAU)
                .ok();
            if element.background_color != "transparent" {
                ctx.fill();
            }
            ctx.stroke();
        }
        ElementType::Line => draw_line(ctx, element, false),
        ElementType::Arrow => draw_line(ctx, element, true),
        ElementType::Freedraw => draw_freedraw(ctx, element),
        ElementType::Text => {
            if let Some(text) = &element.text {
                let size = element.font_size.unwrap_or(20.0);
                ctx.set_font(&format!("{size}px sans-serif"));
                ctx.fill_text(text, element.x, element.y + size).ok();
            }
        }
        _ => {}
    }

    ctx.set_global_alpha(1.0);
}

fn draw_line(ctx: &CanvasRenderingContext2d, element: &Element, arrow: bool) {
    if let Some(points) = &element.points {
        if points.len() >= 4 {
            ctx.begin_path();
            ctx.move_to(points[0], points[1]);
            ctx.line_to(points[2], points[3]);
            ctx.stroke();
            if arrow {
                draw_arrowhead(ctx, points[0], points[1], points[2], points[3]);
            }
        }
    }
}

fn draw_arrowhead(ctx: &CanvasRenderingContext2d, x1: f64, y1: f64, x2: f64, y2: f64) {
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

fn draw_freedraw(ctx: &CanvasRenderingContext2d, element: &Element) {
    if let Some(points) = &element.points {
        if points.len() >= 4 {
            ctx.begin_path();
            ctx.move_to(points[0], points[1]);
            for chunk in points.chunks(2).skip(1) {
                if chunk.len() == 2 {
                    ctx.line_to(chunk[0], chunk[1]);
                }
            }
            ctx.stroke();
        }
    }
}
