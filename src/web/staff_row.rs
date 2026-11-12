use crate::domain::StaffMember;
use crate::error::AppError;

/// One row of the staff catalog, as the template draws it.
#[derive(Debug, Clone)]
pub struct StaffRow {
    /// The member.
    pub member: StaffMember,
    /// Whether a post heading goes above this row.
    pub opens_section: bool,
    /// The name, lowercased, which is what the search box filters on.
    pub search_name: String,
    /// `requires` as JSON, read once by `staff.js`.
    pub requires_json: String,
    /// `excludes` as JSON, read once by `staff.js`.
    pub excludes_json: String,
}

/// The posts, which are the country page's tables, in page order.
pub(crate) fn unique_posts(staff: &[StaffMember]) -> Vec<String> {
    let mut posts: Vec<String> = Vec::new();
    for member in staff {
        if !posts.contains(&member.section) {
            posts.push(member.section.clone());
        }
    }
    posts
}

/// The catalog as rows, each knowing whether a post heading goes above it.
pub(crate) fn staff_rows(staff: Vec<StaffMember>) -> Result<Vec<StaffRow>, AppError> {
    let mut rows: Vec<StaffRow> = Vec::with_capacity(staff.len());
    for member in staff {
        let opens_section = rows
            .last()
            .is_none_or(|previous| previous.member.section != member.section);
        rows.push(StaffRow {
            opens_section,
            search_name: member.name.to_lowercase(),
            requires_json: serde_json::to_string(&member.requires)?,
            excludes_json: serde_json::to_string(&member.excludes)?,
            member,
        });
    }
    Ok(rows)
}
