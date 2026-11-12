/// Where a catalog comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogSource {
    /// Re-parse the saved pages, and rewrite the JSON on the way through.
    WikiPage,
    /// Read the committed JSON: a moment, and no saved page needed.
    Json,
    /// Read the JSON compiled into the binary, which is all a server has.
    Bundled,
}

/// What one run of an import does.
#[derive(Debug, Clone)]
pub struct ImportOptions<'a> {
    /// Where the catalog comes from.
    pub source: CatalogSource,
    /// The database to fill; `None` stops at the JSON.
    pub database_url: Option<&'a str>,
}

/// What one run of an import did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportOutcome {
    /// How many entries were read.
    pub parsed: usize,
    /// How many are stored, when a database was filled.
    pub stored: Option<i64>,
}
