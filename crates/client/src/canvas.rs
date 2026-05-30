use excaildraw_core::{Element, ElementType};
use web_sys::CanvasRenderingContext2d;

use crate::viewport::Viewport;

pub fn render_scene(
    ctx: &CanvasRenderingContext2d,
    viewport: &Viewport,
    width: f64,
    height: f64,
    elements: &[Element],
    preview: Option<&Element>,
) {
    ctx.set_fill_style_str("#f8f9fa");
    ctx.fill_rect(0.0, 0.0, width, height);

    ctx.save();
    ctx.translate(viewport.offset_x, viewport.offset_y).ok();
    ctx.scale(viewport.zoom, viewport.zoom).ok();

    draw_grid(ctx, viewport, width, height);

    for element in elements.iter().filter(|e| !e.is_deleted) {
        draw_element(ctx, element);
    }

    if let Some(preview) = preview {
        ctx.set_global_alpha(0.6);
        draw_element(ctx, preview);
        ctx.set_global_alpha(1.0);
    }

    ctx.restore();
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
            ctx.stroke_rect(element.x, element.y, element.width, element.height);
            if element.background_color != "transparent" {
                ctx.fill_rect(element.x, element.y, element.width, element.height);
            }
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
        ElementType::Line | ElementType::Arrow => {
            if let Some(points) = &element.points {
                if points.len() >= 4 {
                    ctx.begin_path();
                    ctx.move_to(points[0], points[1]);
                    ctx.line_to(points[2], points[3]);
                    ctx.stroke();
                }
            }
        }
        ElementType::Freedraw => {
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
