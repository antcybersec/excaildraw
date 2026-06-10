#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub offset_x: f64,
    pub offset_y: f64,
    pub zoom: f64,
    target_offset_x: f64,
    target_offset_y: f64,
    target_zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            zoom: 1.0,
            target_offset_x: 0.0,
            target_offset_y: 0.0,
            target_zoom: 1.0,
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

    pub fn world_to_screen(&self, wx: f64, wy: f64) -> (f64, f64) {
        (wx * self.zoom + self.offset_x, wy * self.zoom + self.offset_y)
    }

    /// Immediate pan while dragging (no animation lag).
    pub fn pan_immediate(&mut self, dx: f64, dy: f64) {
        self.offset_x += dx;
        self.offset_y += dy;
        self.target_offset_x = self.offset_x;
        self.target_offset_y = self.offset_y;
    }

    /// Smooth zoom toward cursor (Excalidraw-style exponential wheel).
    pub fn zoom_at(&mut self, sx: f64, sy: f64, delta_y: f64) {
        let factor = (-delta_y * 0.0015).exp();
        let new_zoom = (self.target_zoom * factor).clamp(0.05, 32.0);
        let (wx, wy) = Self::screen_to_world_at(
            self.target_offset_x,
            self.target_offset_y,
            self.target_zoom,
            sx,
            sy,
        );
        self.target_zoom = new_zoom;
        self.target_offset_x = sx - wx * new_zoom;
        self.target_offset_y = sy - wy * new_zoom;
    }

    /// Interpolate display values toward targets. Returns true while animating.
    pub fn tick(&mut self, dt: f64) -> bool {
        let t = (1.0 - 0.001_f64.powf(dt * 60.0)).clamp(0.0, 1.0);
        self.offset_x = lerp(self.offset_x, self.target_offset_x, t);
        self.offset_y = lerp(self.offset_y, self.target_offset_y, t);
        self.zoom = lerp(self.zoom, self.target_zoom, t);

        (self.offset_x - self.target_offset_x).abs() > 0.05
            || (self.offset_y - self.target_offset_y).abs() > 0.05
            || (self.zoom - self.target_zoom).abs() > 0.0005
    }

    fn screen_to_world_at(ox: f64, oy: f64, zoom: f64, sx: f64, sy: f64) -> (f64, f64) {
        ((sx - ox) / zoom, (sy - oy) / zoom)
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
