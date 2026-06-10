use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 2;
pub const SOURCE_URL: &str = "https://rustcanvas.app";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElementType {
    Rectangle,
    Ellipse,
    Diamond,
    Arrow,
    Line,
    Freedraw,
    Text,
    Image,
    Frame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FillStyle {
    #[default]
    Solid,
    Hachure,
    #[serde(rename = "cross-hatch")]
    CrossHatch,
    Zigzag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StrokeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Roundness {
    Adaptive { r#type: u8 },
    Proportional { r#type: u8, value: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundElement {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// Canvas element compatible with the Excalidraw JSON schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    pub id: String,
    #[serde(rename = "type")]
    pub element_type: ElementType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub angle: f64,
    #[serde(default = "default_stroke_color")]
    pub stroke_color: String,
    #[serde(default = "default_bg_color")]
    pub background_color: String,
    #[serde(default)]
    pub fill_style: FillStyle,
    #[serde(default = "default_stroke_width")]
    pub stroke_width: f64,
    #[serde(default)]
    pub stroke_style: StrokeStyle,
    #[serde(default)]
    pub roundness: Option<Roundness>,
    #[serde(default = "default_roughness")]
    pub roughness: f64,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    #[serde(default)]
    pub group_ids: Vec<String>,
    #[serde(default)]
    pub frame_id: Option<String>,
    #[serde(default)]
    pub index: Option<String>,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_version")]
    pub version: u64,
    #[serde(default = "default_version_nonce")]
    pub version_nonce: u64,
    #[serde(default)]
    pub is_deleted: bool,
    #[serde(default)]
    pub bound_elements: Option<Vec<BoundElement>>,
    #[serde(default = "now_millis")]
    pub updated: u64,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub locked: bool,
    /// Freehand / line / arrow point data: [x1, y1, x2, y2, ...]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
}

fn default_stroke_color() -> String {
    "#1e1e1e".into()
}

fn default_bg_color() -> String {
    "transparent".into()
}

fn default_stroke_width() -> f64 {
    2.0
}

fn default_roughness() -> f64 {
    0.6
}

fn default_opacity() -> f64 {
    100.0
}

fn default_seed() -> u64 {
    1
}

fn default_version() -> u64 {
    1
}

fn default_version_nonce() -> u64 {
    1
}

fn now_millis() -> u64 {
    0
}

impl Element {
    pub fn new(element_type: ElementType, x: f64, y: f64, width: f64, height: f64) -> Self {
        let seed = random_seed();
        Self {
            id: Uuid::new_v4().to_string(),
            element_type,
            x,
            y,
            width,
            height,
            angle: 0.0,
            stroke_color: default_stroke_color(),
            background_color: default_bg_color(),
            fill_style: FillStyle::Solid,
            stroke_width: default_stroke_width(),
            stroke_style: StrokeStyle::Solid,
            roundness: Some(Roundness::Adaptive { r#type: 3 }),
            roughness: default_roughness(),
            opacity: default_opacity(),
            group_ids: vec![],
            frame_id: None,
            index: None,
            seed,
            version: 1,
            version_nonce: random_nonce(),
            is_deleted: false,
            bound_elements: None,
            updated: current_timestamp(),
            link: None,
            locked: false,
            points: None,
            text: None,
            font_size: None,
            font_family: None,
            text_align: None,
            vertical_align: None,
            file_id: None,
        }
    }

    pub fn bump_version(&mut self) {
        self.version += 1;
        self.version_nonce = random_nonce();
        self.updated = current_timestamp();
    }
}

pub fn random_seed() -> u64 {
    (Uuid::new_v4().as_u128() & u64::MAX as u128) as u64
}

pub fn random_nonce() -> u64 {
    (Uuid::new_v4().as_u128() >> 64) as u64
}

pub fn current_timestamp() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
