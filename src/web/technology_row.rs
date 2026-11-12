use crate::domain::Technology;
use crate::error::AppError;

/// One row of the research catalog, as the template draws it.
#[derive(Debug, Clone)]
pub struct TechnologyRow {
    /// The technology.
    pub technology: Technology,
    /// Whether a research-area heading goes above this row.
    pub opens_domain: bool,
    /// Whether a section heading goes above this row.
    pub opens_section: bool,
    /// The name, lowercased, which is what the search box filters on.
    pub search_name: String,
    /// `requires` as JSON, read once by `technologies.js`.
    pub requires_json: String,
    /// `excludes` as JSON, read once by `technologies.js`.
    pub excludes_json: String,
    /// The year, empty for a secret tech.
    pub year_text: String,
}

/// The research areas, in page order.
pub(crate) fn unique_domains(technologies: &[Technology]) -> Vec<String> {
    let mut domains: Vec<String> = Vec::new();
    for technology in technologies {
        if !domains.contains(&technology.domain) {
            domains.push(technology.domain.clone());
        }
    }
    domains
}

/// The catalog as rows, each knowing which headings go above it.
pub(crate) fn technology_rows(
    technologies: Vec<Technology>,
) -> Result<Vec<TechnologyRow>, AppError> {
    let mut rows: Vec<TechnologyRow> = Vec::with_capacity(technologies.len());
    for technology in technologies {
        let previous = rows.last().map(|row| &row.technology);
        let opens_domain = previous.is_none_or(|other| other.domain != technology.domain);
        let opens_section =
            opens_domain || previous.is_none_or(|other| other.section != technology.section);
        rows.push(TechnologyRow {
            opens_domain,
            opens_section,
            search_name: technology.name.to_lowercase(),
            requires_json: serde_json::to_string(&technology.requires)?,
            excludes_json: serde_json::to_string(&technology.excludes)?,
            year_text: technology
                .year
                .map(|year| year.to_string())
                .unwrap_or_default(),
            technology,
        });
    }
    Ok(rows)
}
