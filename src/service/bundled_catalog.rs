use serde::de::DeserializeOwned;

use crate::error::AppError;

/// A catalog's JSON, compiled into the binary so that a server holding nothing but the binary
/// can be seeded (`seed`).
#[derive(Debug, Clone, Copy)]
pub struct BundledCatalog {
    /// The file of `data/` it was compiled from.
    pub file: &'static str,
    /// Its text.
    pub text: &'static str,
}

/// The focus catalog, `data/focuses.json`.
pub const BUNDLED_FOCUSES: BundledCatalog = BundledCatalog {
    file: "focuses.json",
    text: include_str!("../../data/focuses.json"),
};

/// The research catalog, `data/technologies.json`.
pub const BUNDLED_TECHNOLOGIES: BundledCatalog = BundledCatalog {
    file: "technologies.json",
    text: include_str!("../../data/technologies.json"),
};

/// The staff catalog, `data/staff.json`.
pub const BUNDLED_STAFF: BundledCatalog = BundledCatalog {
    file: "staff.json",
    text: include_str!("../../data/staff.json"),
};

impl BundledCatalog {
    /// Reads the entries of the catalog.
    ///
    /// # Errors
    /// Fails when the compiled-in JSON is not a list of `T`.
    pub fn entries<T: DeserializeOwned>(&self) -> Result<Vec<T>, AppError> {
        serde_json::from_str(self.text).map_err(|source| AppError::InvalidCatalogFile {
            path: format!("{} (intégré au binaire)", self.file),
            source,
        })
    }
}
