use std::collections::HashMap;

use crate::domain::Technology;

/// The technologies keyed by id, which is how a plan's ids are named, in page order.
#[derive(Debug, Clone, Default)]
pub struct TechnologyCatalog {
    entries: Vec<Technology>,
    by_id: HashMap<String, usize>,
}

impl TechnologyCatalog {
    /// Builds the lookup from the catalog's list of technologies, sorted by their place on the page.
    pub fn new(mut technologies: Vec<Technology>) -> TechnologyCatalog {
        technologies.sort_by_key(|technology| technology.position);
        let by_id = technologies
            .iter()
            .enumerate()
            .map(|(index, technology)| (technology.id.clone(), index))
            .collect();
        TechnologyCatalog {
            entries: technologies,
            by_id,
        }
    }

    /// The technology with this id, if the catalog has it.
    pub fn get(&self, technology_id: &str) -> Option<&Technology> {
        self.by_id
            .get(technology_id)
            .and_then(|&index| self.entries.get(index))
    }

    /// Whether the catalog has this id.
    pub fn contains(&self, technology_id: &str) -> bool {
        self.by_id.contains_key(technology_id)
    }

    /// Every entry, in page order.
    pub fn entries(&self) -> &[Technology] {
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
