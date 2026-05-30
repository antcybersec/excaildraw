use crate::element::{Element, ElementType};
use crate::file::ExcalidrawFile;

pub fn to_svg(file: &ExcalidrawFile) -> String {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    let visible: Vec<_> = file.elements.iter().filter(|e| !e.is_deleted).collect();
    if visible.is_empty() {
        return r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>"#.into();
    }

    for element in &visible {
        let (x, y, w, h) = crate::hit_test::element_bounds(element);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    let pad = 20.0;
    min_x -= pad;
    min_y -= pad;
    max_x += pad;
    max_y += pad;
    let width = max_x - min_x;
    let height = max_y - min_y;

    let mut body = String::new();
    for element in visible {
        body.push_str(&element_to_svg(element, min_x, min_y));
    }

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
  <rect width="100%" height="100%" fill="{}"/>
{body}
</svg>"#,
        file.app_state.view_background_color
    )
}

fn element_to_svg(element: &Element, offset_x: f64, offset_y: f64) -> String {
    let x = element.x - offset_x;
    let y = element.y - offset_y;
    let stroke = &element.stroke_color;
    let fill = if element.background_color == "transparent" {
        "none".to_string()
    } else {
        element.background_color.clone()
    };
    let sw = element.stroke_width;
    let opacity = element.opacity / 100.0;

    match element.element_type {
        ElementType::Rectangle => format!(
            r#"  <rect x="{x}" y="{y}" width="{w}" height="{h}" stroke="{stroke}" fill="{fill}" stroke-width="{sw}" opacity="{opacity}"/>"#,
            w = element.width,
            h = element.height
        ),
        ElementType::Ellipse => format!(
            r#"  <ellipse cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}" stroke="{stroke}" fill="{fill}" stroke-width="{sw}" opacity="{opacity}"/>"#,
            cx = x + element.width / 2.0,
            cy = y + element.height / 2.0,
            rx = element.width.abs() / 2.0,
            ry = element.height.abs() / 2.0
        ),
        ElementType::Line | ElementType::Arrow => {
            if let Some(p) = &element.points {
                if p.len() >= 4 {
                    let mut s = format!(
                        r#"  <line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="{sw}" opacity="{opacity}"/>"#,
                        x1 = p[0] - offset_x,
                        y1 = p[1] - offset_y,
                        x2 = p[2] - offset_x,
                        y2 = p[3] - offset_y
                    );
                    if element.element_type == ElementType::Arrow {
                        s.push('\n');
                        s.push_str(&arrowhead_svg(
                            p[0] - offset_x,
                            p[1] - offset_y,
                            p[2] - offset_x,
                            p[3] - offset_y,
                            stroke,
                            sw,
                            opacity,
                        ));
                    }
                    return s;
                }
            }
            String::new()
        }
        ElementType::Freedraw => {
            if let Some(points) = &element.points {
                if points.len() >= 4 {
                    let mut d = format!("M {} {}", points[0] - offset_x, points[1] - offset_y);
                    for chunk in points.chunks(2).skip(1) {
                        if chunk.len() == 2 {
                            d.push_str(&format!(" L {} {}", chunk[0] - offset_x, chunk[1] - offset_y));
                        }
                    }
                    return format!(
                        r#"  <path d="{d}" stroke="{stroke}" fill="none" stroke-width="{sw}" opacity="{opacity}"/>"#
                    );
                }
            }
            String::new()
        }
        ElementType::Text => {
            if let Some(text) = &element.text {
                let size = element.font_size.unwrap_or(20.0);
                let escaped = text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                format!(
                    r#"  <text x="{x}" y="{ty}" font-size="{size}" fill="{stroke}" opacity="{opacity}">{escaped}</text>"#,
                    ty = y + size
                )
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

fn arrowhead_svg(x1: f64, y1: f64, x2: f64, y2: f64, stroke: &str, sw: f64, opacity: f64) -> String {
    let angle = (y2 - y1).atan2(x2 - x1);
    let size = 10.0 + sw;
    let a1 = angle + std::f64::consts::PI * 0.85;
    let a2 = angle - std::f64::consts::PI * 0.85;
    format!(
        r#"  <polygon points="{x2},{y2} {x3},{y3} {x4},{y4}" fill="{stroke}" opacity="{opacity}"/>"#,
        x3 = x2 + size * a1.cos(),
        y3 = y2 + size * a1.sin(),
        x4 = x2 + size * a2.cos(),
        y4 = y2 + size * a2.sin()
    )
}
