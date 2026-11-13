use crate::config::DataFiles;
use crate::domain::Technology;
use crate::error::AppError;
use crate::service::data_file::{read_file, write_json};
use crate::wiki::{TechnologyPageSource, parse_technology_pages};

/// Reads the saved research pages into `data/technologies.json`, which the binary compiles in.
///
/// # Returns
/// How many technologies were read.
///
/// # Errors
/// Fails when a page is missing or unreadable, the pages hold no technology, or the JSON cannot
/// be written.
pub fn parse_technologies(files: &DataFiles) -> Result<usize, AppError> {
    let technologies = read_pages(files)?;
    if technologies.is_empty() {
        return Err(AppError::NothingParsed {
            what: "Aucune technologie",
        });
    }
    write_json(&files.technologies_json, &technologies)?;
    Ok(technologies.len())
}

fn read_pages(files: &DataFiles) -> Result<Vec<Technology>, AppError> {
    let mut pages = Vec::with_capacity(files.technology_pages.len());
    for page in &files.technology_pages {
        pages.push((page.domain, read_file(&page.path)?));
    }
    let sources: Vec<TechnologyPageSource<'_>> = pages
        .iter()
        .map(|(domain, markup)| TechnologyPageSource { domain, markup })
        .collect();
    Ok(parse_technology_pages(&sources))
}
