use crate::element::Element;
use std::collections::HashMap;

/// Merge remote elements into local state using Excalidraw-style last-write-wins rules.
///
/// - Higher `version` wins
/// - Same version → higher `versionNonce` wins
/// - Tombstoned elements (`isDeleted`) are kept for sync but filtered at render time
pub fn reconcile_elements(local: &[Element], remote: &[Element]) -> Vec<Element> {
    let local_map: HashMap<&str, &Element> = local.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut merged: HashMap<String, Element> = HashMap::new();

    for remote_element in remote {
        match local_map.get(remote_element.id.as_str()) {
            Some(local_element) => {
                let winner = pick_winner(local_element, remote_element);
                merged.insert(winner.id.clone(), winner.clone());
            }
            None => {
                merged.insert(remote_element.id.clone(), remote_element.clone());
            }
        }
    }

    for local_element in local {
        merged
            .entry(local_element.id.clone())
            .or_insert_with(|| local_element.clone());
    }

    let mut result: Vec<Element> = merged.into_values().collect();
    sort_by_index(&mut result);
    result
}

fn pick_winner<'a>(local: &'a Element, remote: &'a Element) -> &'a Element {
    if remote.version > local.version {
        return remote;
    }
    if local.version > remote.version {
        return local;
    }
    if remote.version_nonce > local.version_nonce {
        remote
    } else {
        local
    }
}

fn sort_by_index(elements: &mut [Element]) {
    elements.sort_by(|a, b| {
        match (&a.index, &b.index) {
            (Some(ia), Some(ib)) => ia.cmp(ib),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.updated.cmp(&b.updated),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{Element, ElementType};

    fn elem(id: &str, version: u64, nonce: u64) -> Element {
        let mut e = Element::new(ElementType::Rectangle, 0.0, 0.0, 10.0, 10.0);
        e.id = id.into();
        e.version = version;
        e.version_nonce = nonce;
        e
    }

    #[test]
    fn remote_higher_version_wins() {
        let local = vec![elem("a", 1, 100)];
        let remote = vec![elem("a", 2, 50)];
        let merged = reconcile_elements(&local, &remote);
        assert_eq!(merged[0].version, 2);
    }

    #[test]
    fn same_version_higher_nonce_wins() {
        let local = vec![elem("a", 2, 100)];
        let remote = vec![elem("a", 2, 200)];
        let merged = reconcile_elements(&local, &remote);
        assert_eq!(merged[0].version_nonce, 200);
    }

    #[test]
    fn union_of_ids() {
        let local = vec![elem("a", 1, 1)];
        let remote = vec![elem("b", 1, 1)];
        let merged = reconcile_elements(&local, &remote);
        assert_eq!(merged.len(), 2);
    }
}
