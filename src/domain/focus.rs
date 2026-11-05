use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// One national focus of the German tree, keyed by the wiki's own anchor.
///
/// `icon` and `icon_page` are links to the wiki: no icon is ever downloaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Focus {
    /// The wiki anchor, e.g. `Fortify_the_Vaterland`.
    pub id: String,
    /// The name as the page writes it.
    pub name: String,
    /// The wiki's copy of the icon.
    pub icon: String,
    /// The file's page on the wiki.
    pub icon_page: String,
    /// The branch heading the focus sits under.
    pub section: String,
    /// Document order on the page, which keeps a branch's focuses together.
    pub position: i64,
    /// Groups of focus ids: every group needs one of its focuses taken first.
    #[serde(default)]
    pub requires: Vec<Vec<String>>,
    /// The focuses this one cannot share a plan with; both sides carry the other.
    #[serde(default)]
    pub excludes: Vec<String>,
}

impl Focus {
    /// The prerequisite groups `planned` does not satisfy.
    ///
    /// "Requires the following" arrives as one group per focus and "requires one of the
    /// following" as a single group of them all, so this one test covers both.
    pub fn unmet(&self, planned: &HashSet<&str>) -> Vec<Vec<String>> {
        self.requires
            .iter()
            .filter(|group| !group.iter().any(|id| planned.contains(id.as_str())))
            .cloned()
            .collect()
    }

    /// The planned focuses that rule this one out.
    pub fn conflicts(&self, planned: &HashSet<&str>) -> Vec<String> {
        self.excludes
            .iter()
            .filter(|id| planned.contains(id.as_str()))
            .cloned()
            .collect()
    }

    /// Can this focus be taken once `planned` is done? Nothing missing, nothing clashing.
    pub fn available(&self, planned: &HashSet<&str>) -> bool {
        self.unmet(planned).is_empty() && self.conflicts(planned).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gamma(requires: &[&[&str]], excludes: &[&str]) -> Focus {
        Focus {
            id: "Gamma".to_string(),
            name: "Gamma".to_string(),
            icon: String::new(),
            icon_page: String::new(),
            section: String::new(),
            position: 0,
            requires: requires
                .iter()
                .map(|group| group.iter().map(ToString::to_string).collect())
                .collect(),
            excludes: excludes.iter().map(ToString::to_string).collect(),
        }
    }

    fn planned<'a>(ids: &[&'a str]) -> HashSet<&'a str> {
        ids.iter().copied().collect()
    }

    #[test]
    fn unmet_is_empty_once_a_group_has_one_of_its_focuses() {
        let focus = gamma(&[&["Alpha", "Beta"]], &[]);
        assert_eq!(focus.unmet(&planned(&[])), [["Alpha", "Beta"]]);
        assert!(focus.unmet(&planned(&["Beta"])).is_empty());
    }

    #[test]
    fn every_group_has_to_be_met() {
        let focus = gamma(&[&["Alpha"], &["Beta"]], &[]);
        assert_eq!(focus.unmet(&planned(&["Alpha"])), [["Beta"]]);
        assert!(focus.unmet(&planned(&["Alpha", "Beta"])).is_empty());
    }

    #[test]
    fn conflicts_names_the_planned_focus_that_rules_one_out() {
        let focus = gamma(&[], &["Alpha"]);
        assert!(focus.conflicts(&planned(&["Beta"])).is_empty());
        assert_eq!(focus.conflicts(&planned(&["Alpha", "Beta"])), ["Alpha"]);
    }

    #[test]
    fn available_wants_nothing_missing_and_nothing_clashing() {
        let focus = gamma(&[&["Alpha"]], &["Beta"]);
        assert!(!focus.available(&planned(&[])));
        assert!(focus.available(&planned(&["Alpha"])));
        assert!(!focus.available(&planned(&["Alpha", "Beta"])));
    }

    #[test]
    fn a_focus_with_no_dependencies_is_always_available() {
        assert!(gamma(&[], &[]).available(&planned(&[])));
    }

    #[test]
    fn a_catalog_entry_may_leave_its_dependencies_out() {
        let json = r#"{"id": "A", "name": "A", "icon": "i", "icon_page": "p",
                       "section": "S", "position": 0}"#;
        let focus: Focus = serde_json::from_str(json).unwrap();
        assert!(focus.requires.is_empty() && focus.excludes.is_empty());
    }
}
