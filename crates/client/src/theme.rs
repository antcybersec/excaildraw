const THEME_STORAGE_KEY: &str = "rustcanvas-theme";

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
