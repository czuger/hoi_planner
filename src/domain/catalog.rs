use std::collections::HashMap;

use crate::domain::Focus;

/// The focuses keyed by id, which is how a schedule looks them up.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    by_id: HashMap<String, Focus>,
}

impl Catalog {
    /// Builds the lookup from the catalog's list of focuses.
    pub fn new(focuses: Vec<Focus>) -> Catalog {
        Catalog {
            by_id: focuses
                .into_iter()
                .map(|focus| (focus.id.clone(), focus))
                .collect(),
        }
    }

    /// The focus with this id, if the catalog has it.
    pub fn get(&self, focus_id: &str) -> Option<&Focus> {
        self.by_id.get(focus_id)
    }

    /// Whether the catalog has this id.
    pub fn contains(&self, focus_id: &str) -> bool {
        self.by_id.contains_key(focus_id)
    }
}
