use crate::config::DataFiles;
use crate::error::AppError;
use crate::service::data_file::{read_file, write_json};
use crate::wiki::parse_wiki_page;

/// Reads the saved focus page into `data/focuses.json`, which the binary compiles in.
///
/// # Returns
/// How many focuses were read.
///
/// # Errors
/// Fails when the page is missing or unreadable, holds no focus, or the JSON cannot be written.
pub fn parse_focuses(files: &DataFiles) -> Result<usize, AppError> {
    let focuses = parse_wiki_page(&read_file(&files.wiki_page)?);
    if focuses.is_empty() {
        return Err(AppError::NothingParsed {
            what: "Aucun focus",
        });
    }
    write_json(&files.focuses_json, &focuses)?;
    Ok(focuses.len())
}
