use crate::element::Element;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join { user: String, color: String },
    Update { elements: Vec<Element> },
    Cursor { user: String, x: f64, y: f64, color: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Init { elements: Vec<Element> },
    Sync { elements: Vec<Element> },
    Cursor { user: String, x: f64, y: f64, color: String },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collaborator {
    pub user: String,
    pub color: String,
    pub x: f64,
    pub y: f64,
}
