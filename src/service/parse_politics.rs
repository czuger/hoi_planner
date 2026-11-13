use crate::config::DataFiles;
use crate::error::AppError;
use crate::service::data_file::{read_file, write_json};
use crate::wiki::{parse_leaders, parse_staff};

/// How many entries the country page gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoliticsCounts {
    /// The possible country leaders.
    pub leaders: usize,
    /// The staff and designers.
    pub staff: usize,
}

/// Reads the country leaders and the staff out of the saved German Reich page.
///
/// Nothing reads the leaders yet, and the staff are the catalog the binary compiles in. Both
/// are checked before either is written, so a page that lost one of its tables changes nothing.
///
/// # Errors
/// Fails when the page is missing, holds nothing to read, or a file cannot be written.
pub fn parse_politics(files: &DataFiles) -> Result<PoliticsCounts, AppError> {
    let markup = read_file(&files.politics_page)?;
    let leaders = parse_leaders(&markup);
    if leaders.is_empty() {
        return Err(AppError::NothingParsed {
            what: "Aucun dirigeant",
        });
    }
    let staff = parse_staff(&markup);
    if staff.is_empty() {
        return Err(AppError::NothingParsed {
            what: "Aucun membre de l'état-major",
        });
    }
    write_json(&files.leaders_json, &leaders)?;
    write_json(&files.staff_json, &staff)?;
    Ok(PoliticsCounts {
        leaders: leaders.len(),
        staff: staff.len(),
    })
}
