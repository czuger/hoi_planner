use serde::{Deserialize, Serialize};

/// Someone or something the German Reich can hire: an advisor, a commander, a concern or a
/// designer, read from the Personnel and Companies tables of the country page.
///
/// A person listed in two tables is two members, one per post, which is what lets a plan say
/// that Guderian is hired as a theorist rather than as a general.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaffMember {
    /// Made from the name, with the table after it for someone listed in several.
    pub id: String,
    /// The name as the page writes it.
    pub name: String,
    /// The wiki's copy of the portrait or logo, empty when the page shows none.
    #[serde(default, deserialize_with = "empty_when_null")]
    pub icon: String,
    /// The file's page on the wiki, empty when the page shows no picture.
    #[serde(default, deserialize_with = "empty_when_null")]
    pub icon_page: String,
    /// The table it is listed in, which is the post it fills: `Theorists`, `Industrial concerns`.
    pub section: String,
    /// Document order on the page, which keeps a table's rows together.
    pub position: i64,
    /// Groups of focus ids: every group needs one of its focuses taken.
    ///
    /// Not resolved against the focus catalog: many run through branches it does not carry.
    #[serde(default)]
    pub requires: Vec<Vec<String>>,
    /// The focuses that rule this one out.
    #[serde(default)]
    pub excludes: Vec<String>,
}

impl StaffMember {
    /// Whether the member is a designer, a military industrial organization: not staff a line
    /// hires, but what a technology of the plan can be tagged with. Its table says so (`Tank
    /// designers`, `Ship designers`…).
    pub fn is_designer(&self) -> bool {
        self.section.to_lowercase().ends_with(" designers")
    }
}

/// `staff.json` writes a missing picture as `null`.
fn empty_when_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_member_with_no_picture_reads_as_empty_links() {
        let member: StaffMember = serde_json::from_str(
            r#"{"id": "Otto_Strasser", "name": "Otto Strasser", "icon": null, "icon_page": null,
                "section": "Political Advisors", "position": 0,
                "requires": [["Expatriate_the_Communists_B"]], "excludes": []}"#,
        )
        .unwrap();
        assert_eq!(member.icon, "");
        assert_eq!(member.icon_page, "");
        assert_eq!(member.requires, [["Expatriate_the_Communists_B"]]);
    }

    #[test]
    fn only_the_designers_tables_hold_designers() {
        let member = |section: &str| StaffMember {
            id: "X".to_string(),
            name: "X".to_string(),
            icon: String::new(),
            icon_page: String::new(),
            section: section.to_string(),
            position: 0,
            requires: Vec::new(),
            excludes: Vec::new(),
        };
        for section in [
            "Tank designers",
            "Aircraft designers",
            "Materiel designers",
            "Ship designers",
        ] {
            assert!(member(section).is_designer(), "{section}");
        }
        for section in ["Industrial concerns", "Generals", "Theorists"] {
            assert!(!member(section).is_designer(), "{section}");
        }
    }
}
