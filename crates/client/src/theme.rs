#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CanvasTheme {
    pub dark: bool,
}

impl Default for CanvasTheme {
    fn default() -> Self {
        Self { dark: false }
    }
}

impl CanvasTheme {
    pub fn canvas_background(&self) -> &'static str {
        if self.dark {
            "#121212"
        } else {
            "#ffffff"
        }
    }

    pub fn default_stroke(&self) -> &'static str {
        if self.dark {
            "#f0f0f0"
        } else {
            "#1e1e1e"
        }
    }

    pub fn selection_color(&self) -> &'static str {
        "#6965db"
    }
}
