use std::collections::HashMap;

use crate::domain::Focus;

/// The focuses keyed by id, which is how a schedule looks them up, in page order.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    entries: Vec<Focus>,
    by_id: HashMap<String, usize>,
}

impl Catalog {
    /// Builds the lookup from the catalog's list of focuses, sorted by their place on the page.
    pub fn new(mut focuses: Vec<Focus>) -> Catalog {
        focuses.sort_by_key(|focus| focus.position);
        let by_id = focuses
            .iter()
            .enumerate()
            .map(|(index, focus)| (focus.id.clone(), index))
            .collect();
        Catalog {
            entries: focuses,
            by_id,
        }
    }

    /// The focus with this id, if the catalog has it.
    pub fn get(&self, focus_id: &str) -> Option<&Focus> {
        self.by_id
            .get(focus_id)
            .and_then(|&index| self.entries.get(index))
    }

    /// Whether the catalog has this id.
    pub fn contains(&self, focus_id: &str) -> bool {
        self.by_id.contains_key(focus_id)
    }

    /// Every entry, in page order.
    pub fn entries(&self) -> &[Focus] {
        &self.entries
    }

    /// How many entries there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there is none.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus(id: &str, position: i64) -> Focus {
        Focus {
            id: id.to_string(),
            name: id.to_string(),
            icon: String::new(),
            icon_page: String::new(),
            section: "S".to_string(),
            position,
            requires: Vec::new(),
            excludes: Vec::new(),
        }
    }

    #[test]
    fn the_focuses_come_back_in_page_order() {
        let catalog = Catalog::new(vec![focus("C", 2), focus("A", 0), focus("B", 1)]);
        let ids: Vec<&str> = catalog.entries().iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["A", "B", "C"]);
        assert_eq!(catalog.get("C").map(|f| f.position), Some(2));
        assert!(!catalog.contains("D"));
    }
}
