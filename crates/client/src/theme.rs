use excaildraw_core::Element;

const THEME_STORAGE_KEY: &str = "rustcanvas-theme";
const LIGHT_STROKE: &str = "#1e1e1e";
const DARK_STROKE: &str = "#f0f0f0";
const LIGHT_FILL: &str = "#ffffff";
const DARK_FILL: &str = "#2d2d2d";

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CanvasTheme {
    pub dark: bool,
}

impl Default for CanvasTheme {
    fn default() -> Self {
        Self {
            dark: load_pref(),
        }
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

    pub fn toggle(&mut self) {
        self.dark = !self.dark;
        save_pref(self.dark);
        apply_document_class(self.dark);
    }

}

pub fn load_pref() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(THEME_STORAGE_KEY).ok().flatten())
        .is_some_and(|v| v == "dark")
}

pub fn save_pref(dark: bool) {
    if let Some(storage) = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
    {
        let _ = storage.set_item(
            THEME_STORAGE_KEY,
            if dark { "dark" } else { "light" },
        );
    }
}

/// Remap neutral ink/fill colors so existing drawings stay visible after a theme switch.
pub fn adapt_element_colors(el: &mut Element, to_dark: bool) {
    if let Some(stroke) = remap_stroke_for_theme(&el.stroke_color, to_dark) {
        el.stroke_color = stroke;
    }
    if let Some(fill) = remap_fill_for_theme(&el.background_color, to_dark) {
        el.background_color = fill;
    }
}

fn remap_stroke_for_theme(color: &str, to_dark: bool) -> Option<String> {
    if color.eq_ignore_ascii_case("transparent") {
        return None;
    }
    let lum = parse_luminance(color)?;
    if !is_neutral_gray(color) {
        return None;
    }
    if to_dark && lum < 0.45 {
        Some(DARK_STROKE.to_string())
    } else if !to_dark && lum > 0.55 {
        Some(LIGHT_STROKE.to_string())
    } else {
        None
    }
}

fn remap_fill_for_theme(color: &str, to_dark: bool) -> Option<String> {
    if color.eq_ignore_ascii_case("transparent") {
        return None;
    }
    let lum = parse_luminance(color)?;
    if !is_neutral_gray(color) {
        return None;
    }
    if to_dark && lum > 0.85 {
        Some(DARK_FILL.to_string())
    } else if !to_dark && lum < 0.2 {
        Some(LIGHT_FILL.to_string())
    } else {
        None
    }
}

fn parse_luminance(color: &str) -> Option<f64> {
    let hex = color.trim().trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            (r, g, b)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b)
        }
        _ => return None,
    };
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    Some(0.2126 * r + 0.7152 * g + 0.0722 * b)
}

fn is_neutral_gray(color: &str) -> bool {
    let hex = color.trim().trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        3 => (
            u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0),
            u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0),
            u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0),
        ),
        6 => (
            u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
            u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
        ),
        _ => return false,
    };
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    max.saturating_sub(min) <= 30
}

#[cfg(test)]
mod tests {
    use super::*;
    use excaildraw_core::{Element, ElementType};

    #[test]
    fn darkens_light_strokes_when_switching_to_dark() {
        let mut el = Element::new(ElementType::Rectangle, 0.0, 0.0, 10.0, 10.0);
        el.stroke_color = "#1e1e1e".into();
        adapt_element_colors(&mut el, true);
        assert_eq!(el.stroke_color, DARK_STROKE);
    }

    #[test]
    fn lightens_dark_strokes_when_switching_to_light() {
        let mut el = Element::new(ElementType::Line, 0.0, 0.0, 10.0, 10.0);
        el.stroke_color = "#f0f0f0".into();
        adapt_element_colors(&mut el, false);
        assert_eq!(el.stroke_color, LIGHT_STROKE);
    }

    #[test]
    fn keeps_colored_strokes_unchanged() {
        let mut el = Element::new(ElementType::Ellipse, 0.0, 0.0, 10.0, 10.0);
        el.stroke_color = "#e03131".into();
        adapt_element_colors(&mut el, true);
        assert_eq!(el.stroke_color, "#e03131");
    }
}

pub fn apply_document_class(dark: bool) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(html) = doc.document_element() else {
        return;
    };
    if dark {
        let _ = html.set_attribute("class", "dark");
    } else {
        let _ = html.remove_attribute("class");
    }
}
