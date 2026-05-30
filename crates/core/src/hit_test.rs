use crate::element::{Element, ElementType};

pub fn hit_test(elements: &[Element], wx: f64, wy: f64) -> Option<String> {
    for element in elements.iter().rev().filter(|e| !e.is_deleted) {
        if contains_point(element, wx, wy) {
            return Some(element.id.clone());
        }
    }
    None
}

pub fn element_bounds(element: &Element) -> (f64, f64, f64, f64) {
    match &element.points {
        Some(points) if points.len() >= 2 => {
            let mut min_x = points[0];
            let mut min_y = points[1];
            let mut max_x = points[0];
            let mut max_y = points[1];
            for chunk in points.chunks(2) {
                if chunk.len() == 2 {
                    min_x = min_x.min(chunk[0]);
                    min_y = min_y.min(chunk[1]);
                    max_x = max_x.max(chunk[0]);
                    max_y = max_y.max(chunk[1]);
                }
            }
            let pad = element.stroke_width;
            (min_x - pad, min_y - pad, max_x - min_x + pad * 2.0, max_y - min_y + pad * 2.0)
        }
        _ => (element.x, element.y, element.width, element.height),
    }
}

fn contains_point(element: &Element, wx: f64, wy: f64) -> bool {
    match element.element_type {
        ElementType::Rectangle | ElementType::Text | ElementType::Diamond | ElementType::Frame => {
            let pad = element.stroke_width + 4.0;
            wx >= element.x - pad
                && wx <= element.x + element.width + pad
                && wy >= element.y - pad
                && wy <= element.y + element.height + pad
        }
        ElementType::Ellipse => {
            let cx = element.x + element.width / 2.0;
            let cy = element.y + element.height / 2.0;
            let rx = element.width.abs() / 2.0 + 4.0;
            let ry = element.height.abs() / 2.0 + 4.0;
            if rx <= 0.0 || ry <= 0.0 {
                return false;
            }
            let dx = wx - cx;
            let dy = wy - cy;
            (dx * dx) / (rx * rx) + (dy * dy) / (ry * ry) <= 1.0
        }
        ElementType::Line | ElementType::Arrow | ElementType::Freedraw => {
            if let Some(points) = &element.points {
                return near_polyline(points, wx, wy, element.stroke_width + 6.0);
            }
            false
        }
        _ => false,
    }
}

fn near_polyline(points: &[f64], wx: f64, wy: f64, threshold: f64) -> bool {
    for window in points.windows(4) {
        if distance_to_segment(wx, wy, window[0], window[1], window[2], window[3]) <= threshold {
            return true;
        }
    }
    false
}

fn distance_to_segment(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    if dx == 0.0 && dy == 0.0 {
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }
    let t = ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy);
    let t = t.clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{Element, ElementType};

    #[test]
    fn hits_rectangle() {
        let e = Element::new(ElementType::Rectangle, 10.0, 10.0, 50.0, 30.0);
        assert!(contains_point(&e, 20.0, 20.0));
        assert!(!contains_point(&e, 0.0, 0.0));
    }
}
