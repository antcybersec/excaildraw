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
        let r = roughness.max(0.1);
        (self.next_f64() - 0.5) * r * stroke_width * 2.5
    }
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
    let mut rng = SeededRng::new(seed);
    let w = width.max(1.0);
    let h = height.max(1.0);
    let corners = [
        (x, y),
        (x + w, y),
        (x + w, y + h),
        (x, y + h),
    ];
    let mut points = Vec::with_capacity(5);
    for (cx, cy) in corners {
        points.push((
            cx + rng.offset(roughness, stroke_width),
            cy + rng.offset(roughness, stroke_width),
        ));
    }
    points.push(points[0]);
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
    let mut rng = SeededRng::new(seed.wrapping_add(17));
    let steps = 32;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = (i as f64 / steps as f64) * std::f64::consts::TAU;
        let px = cx + rx * t.cos() + rng.offset(roughness, stroke_width);
        let py = cy + ry * t.sin() + rng.offset(roughness, stroke_width);
        points.push((px, py));
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
    let mut rng = SeededRng::new(seed.wrapping_add(31));
    let mut points = Vec::new();
    for chunk in coords.chunks(2) {
        if chunk.len() == 2 {
            points.push((
                chunk[0] + rng.offset(roughness, stroke_width),
                chunk[1] + rng.offset(roughness, stroke_width),
            ));
        }
    }
    if closed && !points.is_empty() {
        points.push(points[0]);
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
}
