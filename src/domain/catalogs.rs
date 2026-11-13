use crate::domain::{Catalog, StaffCatalog, TechnologyCatalog};

/// The three catalogs a plan is drawn from and named by.
#[derive(Debug, Clone, Default)]
pub struct Catalogs {
    /// The national focuses.
    pub focuses: Catalog,
    /// The research tree.
    pub technologies: TechnologyCatalog,
    /// The staff and designers.
    pub staff: StaffCatalog,
}
