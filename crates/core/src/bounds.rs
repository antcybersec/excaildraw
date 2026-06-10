use crate::element::{Element, ElementType};

pub fn normalize_element_bounds(el: &mut Element) {
    match el.element_type {
        ElementType::Line | ElementType::Arrow | ElementType::Freedraw => {
            if let Some(points) = &el.points {
                if points.len() < 2 {
                    return;
                }
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
                el.x = min_x;
                el.y = min_y;
                el.width = (max_x - min_x).max(1.0);
                el.height = (max_y - min_y).max(1.0);
            }
        }
        _ => {}
    }
}
