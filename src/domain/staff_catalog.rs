use std::collections::HashMap;

use crate::domain::StaffMember;

/// The staff keyed by id, which is how a plan's staff ids are named, in page order.
#[derive(Debug, Clone, Default)]
pub struct StaffCatalog {
    entries: Vec<StaffMember>,
    by_id: HashMap<String, usize>,
}

impl StaffCatalog {
    /// Builds the lookup from the catalog's list of members, sorted by their place on the page.
    pub fn new(mut members: Vec<StaffMember>) -> StaffCatalog {
        members.sort_by_key(|member| member.position);
        let by_id = members
            .iter()
            .enumerate()
            .map(|(index, member)| (member.id.clone(), index))
            .collect();
        StaffCatalog {
            entries: members,
            by_id,
        }
    }

    /// The member with this id, if the catalog has it.
    pub fn get(&self, staff_id: &str) -> Option<&StaffMember> {
        self.by_id
            .get(staff_id)
            .and_then(|&index| self.entries.get(index))
    }

    /// Whether the catalog has this id.
    pub fn contains(&self, staff_id: &str) -> bool {
        self.by_id.contains_key(staff_id)
    }

    /// Every entry, in page order.
    pub fn entries(&self) -> &[StaffMember] {
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
