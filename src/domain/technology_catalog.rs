use std::collections::HashMap;

use crate::domain::Technology;

/// The technologies keyed by id, which is how a plan's ids are named.
#[derive(Debug, Clone, Default)]
pub struct TechnologyCatalog {
    by_id: HashMap<String, Technology>,
}

impl TechnologyCatalog {
    /// Builds the lookup from the catalog's list of technologies.
    pub fn new(technologies: Vec<Technology>) -> TechnologyCatalog {
        TechnologyCatalog {
            by_id: technologies
                .into_iter()
                .map(|technology| (technology.id.clone(), technology))
                .collect(),
        }
    }

    /// The technology with this id, if the catalog has it.
    pub fn get(&self, technology_id: &str) -> Option<&Technology> {
        self.by_id.get(technology_id)
    }

    /// Whether the catalog has this id.
    pub fn contains(&self, technology_id: &str) -> bool {
        self.by_id.contains_key(technology_id)
    }
}
