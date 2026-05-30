use excaildraw_core::{Element, History, hit_test};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Freedraw,
    Text,
}

pub struct Editor {
    pub elements: Vec<Element>,
    pub history: History,
    pub selected: Option<String>,
    pub dragging: Option<(String, f64, f64)>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            history: History::new(),
            selected: None,
            dragging: None,
        }
    }
}

impl Editor {
    pub fn snapshot(&self) -> Vec<Element> {
        self.elements.clone()
    }

    pub fn record_history(&mut self) {
        self.history.record(self.snapshot());
    }

    pub fn undo(&mut self) -> bool {
        self.history.undo(&mut self.elements)
    }

    pub fn redo(&mut self) -> bool {
        self.history.redo(&mut self.elements)
    }

    pub fn set_elements(&mut self, elements: Vec<Element>) {
        self.elements = elements;
        self.selected = None;
    }

    pub fn push_element(&mut self, element: Element) {
        self.record_history();
        self.elements.push(element);
    }

    pub fn delete_selected(&mut self) -> bool {
        let Some(id) = self.selected.clone() else {
            return false;
        };
        self.record_history();
        if let Some(el) = self.elements.iter_mut().find(|e| e.id == id) {
            el.is_deleted = true;
            el.bump_version();
        }
        self.selected = None;
        true
    }

    pub fn start_drag(&mut self, wx: f64, wy: f64) -> bool {
        if let Some(id) = hit_test(&self.elements, wx, wy) {
            self.selected = Some(id.clone());
            self.dragging = Some((id, wx, wy));
            true
        } else {
            self.selected = None;
            false
        }
    }

    pub fn drag_to(&mut self, wx: f64, wy: f64) {
        let Some((id, ox, oy)) = self.dragging.clone() else {
            return;
        };
        let dx = wx - ox;
        let dy = wy - oy;
        if let Some(el) = self.elements.iter_mut().find(|e| e.id == id) {
            el.x += dx;
            el.y += dy;
            if let Some(points) = el.points.as_mut() {
                for chunk in points.chunks_mut(2) {
                    if chunk.len() == 2 {
                        chunk[0] += dx;
                        chunk[1] += dy;
                    }
                }
            }
            el.bump_version();
        }
        self.dragging = Some((id, wx, wy));
    }

    pub fn end_drag(&mut self) {
        if self.dragging.take().is_some() {
            self.record_history();
        }
    }

    pub fn merge_remote(&mut self, remote: Vec<Element>) {
        self.elements = excaildraw_core::reconcile_elements(&self.elements, &remote);
    }
}

pub fn user_color(name: &str) -> String {
    const COLORS: [&str; 8] = [
        "#e03131", "#6741d9", "#1971c2", "#2f9e44", "#f08c00", "#e64980", "#099268", "#495057",
    ];
    let idx = name.bytes().map(|b| b as usize).sum::<usize>() % COLORS.len();
    COLORS[idx].to_string()
}

pub type Collaborators = HashMap<String, (f64, f64, String)>;
