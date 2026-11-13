use serde::de::DeserializeOwned;

use crate::domain::{Catalog, Catalogs, StaffCatalog, TechnologyCatalog};
use crate::error::AppError;

/// A catalog's JSON, compiled into the binary: the app serves it, and needs nothing else.
#[derive(Debug, Clone, Copy)]
struct BundledCatalog {
    /// The file of `data/` it was compiled from.
    file: &'static str,
    /// Its text.
    text: &'static str,
}

/// The focus catalog, `data/focuses.json`.
const BUNDLED_FOCUSES: BundledCatalog = BundledCatalog {
    file: "focuses.json",
    text: include_str!("../../data/focuses.json"),
};

/// The research catalog, `data/technologies.json`.
const BUNDLED_TECHNOLOGIES: BundledCatalog = BundledCatalog {
    file: "technologies.json",
    text: include_str!("../../data/technologies.json"),
};

/// The staff catalog, `data/staff.json`.
const BUNDLED_STAFF: BundledCatalog = BundledCatalog {
    file: "staff.json",
    text: include_str!("../../data/staff.json"),
};

impl BundledCatalog {
    /// Reads the entries of the catalog.
    ///
    /// # Errors
    /// Fails when the compiled-in JSON is not a list of `T`.
    fn entries<T: DeserializeOwned>(&self) -> Result<Vec<T>, AppError> {
        serde_json::from_str(self.text).map_err(|source| AppError::InvalidCatalogFile {
            path: format!("{} (intégré au binaire)", self.file),
            source,
        })
    }
}

/// The three catalogs compiled into the binary, which is what the app serves.
///
/// # Errors
/// Fails when one of them is not a list of what it should hold.
pub fn bundled_catalogs() -> Result<Catalogs, AppError> {
    Ok(Catalogs {
        focuses: Catalog::new(BUNDLED_FOCUSES.entries()?),
        technologies: TechnologyCatalog::new(BUNDLED_TECHNOLOGIES.entries()?),
        staff: StaffCatalog::new(BUNDLED_STAFF.entries()?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compiled_in_catalogs_are_read_whole() {
        let catalogs = bundled_catalogs().unwrap();
        assert!(catalogs.focuses.len() > 100);
        assert!(catalogs.technologies.len() > 100);
        assert!(catalogs.staff.len() > 100);
    }

    #[test]
    fn the_compiled_in_catalogs_keep_the_order_of_the_pages() {
        let catalogs = bundled_catalogs().unwrap();
        let positions: Vec<i64> = catalogs
            .focuses
            .entries()
            .iter()
            .map(|f| f.position)
            .collect();
        assert!(positions.is_sorted());
    }
}
