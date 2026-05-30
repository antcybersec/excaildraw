use crate::element::Element;

const DEFAULT_LIMIT: usize = 100;

/// Undo/redo stack storing element snapshots.
#[derive(Debug, Clone, Default)]
pub struct History {
    past: Vec<Vec<Element>>,
    future: Vec<Vec<Element>>,
    limit: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            past: Vec::new(),
            future: Vec::new(),
            limit: DEFAULT_LIMIT,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn record(&mut self, snapshot: Vec<Element>) {
        if self.past.len() >= self.limit {
            self.past.remove(0);
        }
        self.past.push(snapshot);
        self.future.clear();
    }

    pub fn undo(&mut self, current: &mut Vec<Element>) -> bool {
        let Some(previous) = self.past.pop() else {
            return false;
        };
        self.future.push(std::mem::take(current));
        *current = previous;
        true
    }

    pub fn redo(&mut self, current: &mut Vec<Element>) -> bool {
        let Some(next) = self.future.pop() else {
            return false;
        };
        self.past.push(std::mem::take(current));
        *current = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{Element, ElementType};

    #[test]
    fn undo_redo_roundtrip() {
        let mut history = History::new();
        let mut elements = vec![Element::new(ElementType::Rectangle, 0.0, 0.0, 10.0, 10.0)];
        history.record(elements.clone());
        elements.push(Element::new(ElementType::Ellipse, 1.0, 1.0, 5.0, 5.0));
        assert!(history.undo(&mut elements));
        assert_eq!(elements.len(), 1);
        assert!(history.redo(&mut elements));
        assert_eq!(elements.len(), 2);
    }
}
