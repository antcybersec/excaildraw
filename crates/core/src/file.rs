use crate::element::{Element, SCHEMA_VERSION, SOURCE_URL};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    #[serde(default = "default_bg")]
    pub view_background_color: String,
    #[serde(default = "default_grid")]
    pub grid_size: Option<f64>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

fn default_bg() -> String {
    "#ffffff".into()
}

fn default_grid() -> Option<f64> {
    Some(20.0)
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            view_background_color: default_bg(),
            grid_size: default_grid(),
            extra: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcalidrawFile {
    #[serde(rename = "type")]
    pub file_type: String,
    pub version: u32,
    pub source: String,
    pub elements: Vec<Element>,
    #[serde(default)]
    pub app_state: AppState,
    #[serde(default)]
    pub files: HashMap<String, Value>,
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error("invalid file type: expected excaildraw/excalidraw")]
    InvalidType,
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ExcalidrawFile {
    pub fn new(elements: Vec<Element>) -> Self {
        Self {
            file_type: "excalidraw".into(),
            version: SCHEMA_VERSION,
            source: SOURCE_URL.into(),
            elements,
            app_state: AppState::default(),
            files: HashMap::new(),
        }
    }

    pub fn from_json(json: &str) -> Result<Self, FileError> {
        let file: Self = serde_json::from_str(json)?;
        if file.file_type != "excalidraw" && file.file_type != "excaildraw" {
            return Err(FileError::InvalidType);
        }
        Ok(file)
    }

    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
