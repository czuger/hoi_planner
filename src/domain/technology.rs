use serde::{Deserialize, Serialize};

/// One technology of the research tree, keyed by the wiki's own anchor.
///
/// Country-agnostic: the pages describe the tree every nation shares. `icon` and `icon_page`
/// are links to the wiki, as for a focus: no icon is ever downloaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Technology {
    /// The wiki anchor, e.g. `Construction_I`.
    pub id: String,
    /// The name as the page writes it.
    pub name: String,
    /// The wiki's copy of the icon.
    pub icon: String,
    /// The file's page on the wiki.
    pub icon_page: String,
    /// The research area, which is the page it comes from.
    pub domain: String,
    /// The heading it sits under on that page.
    pub section: String,
    /// Order across the pages, which keeps an area's sections together.
    pub position: i64,
    /// The year the game dates it; secret and special-project techs have none.
    #[serde(default)]
    pub year: Option<i64>,
    /// What it costs to research, in days; secret and special-project techs say nothing.
    #[serde(default)]
    pub base_days: Option<i64>,
    /// Groups of technology ids: every group needs one of its technologies taken first.
    #[serde(default)]
    pub requires: Vec<Vec<String>>,
    /// The technologies this one cannot share a plan with; both sides carry the other.
    #[serde(default)]
    pub excludes: Vec<String>,
}
