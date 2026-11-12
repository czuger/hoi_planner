use crate::domain::{Catalog, PlanFields, PlanFile, StaffCatalog, TechnologyCatalog, slugify};
use crate::error::AppError;

/// Two extensions on purpose: the file is JSON and opens as JSON, the middle one says whose.
const FILE_SUFFIX: &str = ".hoi4plan.json";

/// One plan as the file a browser saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanDownload {
    /// Indented so that it can be read and diffed, with accents left as they are.
    pub body: String,
    /// The plan's name slugged, as it has to survive a file name; `plan` if nothing is left.
    pub filename: String,
}

impl PlanDownload {
    /// Writes the file of one plan.
    ///
    /// # Errors
    /// Fails if the plan cannot be written as JSON.
    pub fn new(
        name: Option<String>,
        fields: PlanFields,
        catalog: &Catalog,
        technologies: &TechnologyCatalog,
        staff: &StaffCatalog,
    ) -> Result<PlanDownload, AppError> {
        let slug = slugify(name.as_deref().unwrap_or(""));
        let stem = if slug.is_empty() { "plan" } else { &slug };
        let filename = format!("{stem}{FILE_SUFFIX}");
        let file = PlanFile::new(name, fields, catalog, technologies, staff);
        Ok(PlanDownload {
            body: serde_json::to_string_pretty(&file)? + "\n",
            filename,
        })
    }
}
