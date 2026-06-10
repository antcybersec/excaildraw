//! Hand-drawn path generation inspired by rough.js (simplified).

/// Deterministic pseudo-random generator (mulberry32).
pub struct SeededRng {
    state: u32,
}

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed as u32 ^ (seed >> 32) as u32,
        }
    }

    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(1_664_525).wrapping_add(1_013_904_422);
        let t = self.state;
        let t = (t ^ (t >> 15)).wrapping_mul(1 | t);
        let t = (t ^ (t >> 13)).wrapping_mul(1 | t);
        (t ^ (t >> 16)) as f64 / u32::MAX as f64
    }

    fn offset(&mut self, roughness: f64, stroke_width: f64) -> f64 {
        let r = roughness.max(0.0);
        (self.next_f64() - 0.5) * r * stroke_width * 0.85
    }
}

fn rough_line_segment(
    seed: u64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    roughness: f64,
    stroke_width: f64,
) -> Vec<(f64, f64)> {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return vec![(x1, y1)];
    }

    if roughness < 0.01 {
        return vec![(x1, y1), (x2, y2)];
    }

    let mut rng = SeededRng::new(seed);
    let segments = ((len / 10.0).ceil() as usize).clamp(2, 16);
    let nx = -dy / len;
    let ny = dx / len;

    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let px = x1 + dx * t;
        let py = y1 + dy * t;
        let edge_off = if i == 0 || i == segments {
            rng.offset(roughness * 0.35, stroke_width)
        } else {
            rng.offset(roughness, stroke_width)
        };
        points.push((px + nx * edge_off, py + ny * edge_off));
    }
    points
}

pub fn rough_rectangle(
    seed: u64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    roughness: f64,
    stroke_width: f64,
) -> Vec<(f64, f64)> {
    let w = width.max(1.0);
    let h = height.max(1.0);

    if roughness < 0.01 {
        return vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h), (x, y)];
    }

    let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    let mut points = Vec::new();
    for i in 0..4 {
        let (x1, y1) = corners[i];
        let (x2, y2) = corners[(i + 1) % 4];
        let edge = rough_line_segment(
            seed.wrapping_add(i as u64 * 17),
            x1,
            y1,
            x2,
            y2,
            roughness,
            stroke_width,
        );
        if i == 0 {
            points.extend(edge);
        } else {
            points.extend(edge.into_iter().skip(1));
        }
    }
    if let Some(first) = points.first().copied() {
        points.push(first);
    }
    points
}

pub fn rough_ellipse(
    seed: u64,
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    roughness: f64,
    stroke_width: f64,
) -> Vec<(f64, f64)> {
    let rx = rx.max(0.5);
    let ry = ry.max(0.5);
    let steps = 64;

    if roughness < 0.01 {
        let mut points = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let t = (i as f64 / steps as f64) * std::f64::consts::TAU;
            points.push((cx + rx * t.cos(), cy + ry * t.sin()));
        }
        return points;
    }

    let mut rng = SeededRng::new(seed.wrapping_add(17));
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = (i as f64 / steps as f64) * std::f64::consts::TAU;
        let px = cx + rx * t.cos();
        let py = cy + ry * t.sin();
        let nx = t.cos();
        let ny = t.sin();
        let off = rng.offset(roughness * 0.65, stroke_width);
        points.push((px + nx * off, py + ny * off));
    }
    points
}

pub fn rough_polyline(
    seed: u64,
    coords: &[f64],
    roughness: f64,
    stroke_width: f64,
    closed: bool,
) -> Vec<(f64, f64)> {
    if coords.len() < 4 {
        return Vec::new();
    }

    let mut points = Vec::new();
    let pairs: Vec<(f64, f64)> = coords
        .chunks(2)
        .filter_map(|c| (c.len() == 2).then_some((c[0], c[1])))
        .collect();

    if pairs.len() < 2 {
        return points;
    }

    let segment_count = if closed { pairs.len() } else { pairs.len() - 1 };
    for i in 0..segment_count {
        let (x1, y1) = pairs[i];
        let (x2, y2) = if i + 1 < pairs.len() {
            pairs[i + 1]
        } else {
            pairs[0]
        };
        let edge = rough_line_segment(
            seed.wrapping_add(i as u64 * 23),
            x1,
            y1,
            x2,
            y2,
            roughness,
            stroke_width,
        );
        if points.is_empty() {
            points.extend(edge);
        } else {
            points.extend(edge.into_iter().skip(1));
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_paths() {
        let a = rough_rectangle(42, 0.0, 0.0, 100.0, 50.0, 1.0, 2.0);
        let b = rough_rectangle(42, 0.0, 0.0, 100.0, 50.0, 1.0, 2.0);
        assert_eq!(a, b);
        assert!(a.len() >= 4);
    }

    #[test]
    fn rectangle_stays_axis_aligned_at_zero_roughness() {
        let path = rough_rectangle(1, 10.0, 20.0, 80.0, 40.0, 0.0, 2.0);
        assert_eq!(path[0], (10.0, 20.0));
        assert_eq!(path[1], (90.0, 20.0));
        assert_eq!(path[2], (90.0, 60.0));
        assert_eq!(path[3], (10.0, 60.0));
    }
}
