use excaildraw_core::{Element, History, hit_test};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Freedraw,
    Text,
    Image,
}

pub struct Editor {
    pub elements: Vec<Element>,
    pub files: HashMap<String, Value>,
    pub history: History,
    pub selected: HashSet<String>,
    pub dragging: Option<(String, f64, f64)>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            files: HashMap::new(),
            history: History::new(),
            selected: HashSet::new(),
            dragging: None,
        }
    }
}

impl Editor {
    pub fn visible_elements(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter().filter(|e| !e.is_deleted)
    }

    pub fn layer_list(&self) -> Vec<(String, String)> {
        self.visible_elements()
            .map(|e| (e.id.clone(), layer_label(e)))
            .collect()
    }

    pub fn snapshot(&self) -> Vec<Element> {
        self.elements.clone()
    }

    pub fn record_history(&mut self) {
        self.history.record(self.snapshot());
    }

    pub fn undo(&mut self) -> bool {
        let ok = self.history.undo(&mut self.elements);
        self.selected.clear();
        ok
    }

    pub fn redo(&mut self) -> bool {
        let ok = self.history.redo(&mut self.elements);
        self.selected.clear();
        ok
    }

    pub fn load_scene(&mut self, elements: Vec<Element>, files: HashMap<String, Value>) {
        self.elements = elements;
        self.files = files;
        self.selected.clear();
    }

    pub fn push_element(&mut self, element: Element) {
        self.record_history();
        self.elements.push(element);
    }

    pub fn add_file(&mut self, file_id: String, data: Value) {
        self.files.insert(file_id, data);
    }

    pub fn delete_selected(&mut self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        self.record_history();
        for id in &self.selected {
            if let Some(el) = self.elements.iter_mut().find(|e| e.id == *id) {
                el.is_deleted = true;
                el.bump_version();
            }
        }
        self.selected.clear();
        true
    }

    pub fn select_at(&mut self, wx: f64, wy: f64, additive: bool) {
        if let Some(id) = hit_test(&self.elements, wx, wy) {
            if additive {
                if self.selected.contains(&id) {
                    self.selected.remove(&id);
                } else {
                    self.selected.insert(id);
                }
            } else {
                self.selected.clear();
                self.selected.insert(id);
            }
        } else if !additive {
            self.selected.clear();
        }
    }

    pub fn start_drag(&mut self, wx: f64, wy: f64, additive: bool) -> bool {
        self.select_at(wx, wy, additive);
        if let Some(id) = self.primary_selection() {
            self.dragging = Some((id, wx, wy));
            true
        } else {
            false
        }
    }

    pub fn primary_selection(&self) -> Option<String> {
        self.selected.iter().next().cloned()
    }

    pub fn drag_to(&mut self, wx: f64, wy: f64) {
        let Some((id, ox, oy)) = self.dragging.clone() else {
            return;
        };
        let dx = wx - ox;
        let dy = wy - oy;
        for sel in &self.selected.clone() {
            if let Some(el) = self.elements.iter_mut().find(|e| e.id == *sel) {
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
        }
        self.dragging = Some((id, wx, wy));
    }

    pub fn end_drag(&mut self) {
        if self.dragging.take().is_some() {
            self.record_history();
        }
    }

    pub fn move_layer(&mut self, id: &str, direction: i32) {
        let idx = self.elements.iter().position(|e| e.id == id);
        let Some(idx) = idx else { return };
        let new_idx = (idx as i32 + direction).clamp(0, self.elements.len() as i32 - 1) as usize;
        if idx == new_idx {
            return;
        }
        self.record_history();
        let el = self.elements.remove(idx);
        self.elements.insert(new_idx, el);
    }

    pub fn merge_remote(&mut self, remote: Vec<Element>) {
        self.elements = excaildraw_core::reconcile_elements(&self.elements, &remote);
    }
}

fn layer_label(e: &Element) -> String {
    use excaildraw_core::ElementType;
    match e.element_type {
        ElementType::Rectangle => "Rectangle".into(),
        ElementType::Ellipse => "Ellipse".into(),
        ElementType::Line => "Line".into(),
        ElementType::Arrow => "Arrow".into(),
        ElementType::Freedraw => "Freehand".into(),
        ElementType::Text => e.text.clone().unwrap_or_else(|| "Text".into()),
        ElementType::Image => "Image".into(),
        ElementType::Diamond => "Diamond".into(),
        ElementType::Frame => "Frame".into(),
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
