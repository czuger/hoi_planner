use std::collections::HashMap;

use crate::domain::StaffMember;

/// The staff keyed by id, which is how a plan's staff ids are named.
#[derive(Debug, Clone, Default)]
pub struct StaffCatalog {
    by_id: HashMap<String, StaffMember>,
}

impl StaffCatalog {
    /// Builds the lookup from the catalog's list of members.
    pub fn new(members: Vec<StaffMember>) -> StaffCatalog {
        StaffCatalog {
            by_id: members
                .into_iter()
                .map(|member| (member.id.clone(), member))
                .collect(),
        }
    }

    /// The member with this id, if the catalog has it.
    pub fn get(&self, staff_id: &str) -> Option<&StaffMember> {
        self.by_id.get(staff_id)
    }

    /// Whether the catalog has this id.
    pub fn contains(&self, staff_id: &str) -> bool {
        self.by_id.contains_key(staff_id)
    }
}
