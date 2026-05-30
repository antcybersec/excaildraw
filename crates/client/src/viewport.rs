#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub offset_x: f64,
    pub offset_y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Viewport {
    pub fn screen_to_world(&self, sx: f64, sy: f64) -> (f64, f64) {
        (
            (sx - self.offset_x) / self.zoom,
            (sy - self.offset_y) / self.zoom,
        )
    }

    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.offset_x += dx;
        self.offset_y += dy;
    }

    pub fn zoom_at(&mut self, sx: f64, sy: f64, delta: f64) {
        let factor = if delta > 0.0 { 1.1 } else { 0.9 };
        let new_zoom = (self.zoom * factor).clamp(0.1, 8.0);
        let (wx, wy) = self.screen_to_world(sx, sy);
        self.zoom = new_zoom;
        self.offset_x = sx - wx * self.zoom;
        self.offset_y = sy - wy * self.zoom;
    }
}
