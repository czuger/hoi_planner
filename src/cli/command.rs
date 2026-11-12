use crate::error::AppError;
use crate::service::CatalogSource;

const USAGE: &str = "Usage : hoi4_planner [serve | healthcheck | parse-politics | import-focuses [--from-json] [--json-only] [--database-url URL] | import-technologies [--from-json] [--json-only] [--database-url URL] | import-staff [--from-json] [--json-only] [--database-url URL]]";

/// What one run of the binary is asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Serves the app, which is what no argument at all means.
    Serve,
    /// Asks a running server for `/health`.
    Healthcheck,
    /// Reads `data/politics.html` into its two JSON files.
    ParsePolitics,
    /// Fills the catalog.
    ImportFocuses {
        /// Where the catalog comes from.
        source: CatalogSource,
        /// Stop at the JSON, touching no database.
        json_only: bool,
        /// The database to fill, instead of `DATABASE_URL`'s.
        database_url: Option<String>,
    },
    /// Fills the technology catalog.
    ImportTechnologies {
        /// Where the catalog comes from.
        source: CatalogSource,
        /// Stop at the JSON, touching no database.
        json_only: bool,
        /// The database to fill, instead of `DATABASE_URL`'s.
        database_url: Option<String>,
    },
    /// Fills the staff catalog.
    ImportStaff {
        /// Where the catalog comes from.
        source: CatalogSource,
        /// Stop at the JSON, touching no database.
        json_only: bool,
        /// The database to fill, instead of `DATABASE_URL`'s.
        database_url: Option<String>,
    },
}

impl Command {
    /// Reads the command line, without the program's name.
    ///
    /// # Errors
    /// Returns the usage when the arguments make no sense.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, AppError> {
        let mut args = args.into_iter();
        let Some(name) = args.next() else {
            return Ok(Command::Serve);
        };
        let rest: Vec<String> = args.collect();
        match name.as_str() {
            "serve" if rest.is_empty() => Ok(Command::Serve),
            "healthcheck" if rest.is_empty() => Ok(Command::Healthcheck),
            "parse-politics" if rest.is_empty() => Ok(Command::ParsePolitics),
            "import-focuses" => {
                let (source, json_only, database_url) = parse_import(rest)?;
                Ok(Command::ImportFocuses {
                    source,
                    json_only,
                    database_url,
                })
            }
            "import-technologies" => {
                let (source, json_only, database_url) = parse_import(rest)?;
                Ok(Command::ImportTechnologies {
                    source,
                    json_only,
                    database_url,
                })
            }
            "import-staff" => {
                let (source, json_only, database_url) = parse_import(rest)?;
                Ok(Command::ImportStaff {
                    source,
                    json_only,
                    database_url,
                })
            }
            _ => Err(usage()),
        }
    }
}

/// The flags every import takes: the source, whether to stop at the JSON, the database.
fn parse_import(args: Vec<String>) -> Result<(CatalogSource, bool, Option<String>), AppError> {
    let mut source = CatalogSource::WikiPage;
    let mut json_only = false;
    let mut database_url = None;
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--from-json" => source = CatalogSource::Json,
            "--json-only" => json_only = true,
            "--database-url" => database_url = Some(args.next().ok_or_else(usage)?),
            _ => return Err(usage()),
        }
    }
    Ok((source, json_only, database_url))
}

fn usage() -> AppError {
    AppError::Usage(USAGE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, AppError> {
        Command::parse(args.iter().map(ToString::to_string))
    }

    #[test]
    fn no_argument_serves() {
        assert_eq!(parse(&[]).unwrap(), Command::Serve);
        assert_eq!(parse(&["serve"]).unwrap(), Command::Serve);
    }

    #[test]
    fn the_import_reads_the_page_unless_told_otherwise() {
        assert_eq!(
            parse(&["import-focuses"]).unwrap(),
            Command::ImportFocuses {
                source: CatalogSource::WikiPage,
                json_only: false,
                database_url: None,
            }
        );
    }

    #[test]
    fn the_import_takes_its_flags() {
        assert_eq!(
            parse(&["import-focuses", "--from-json", "--database-url", "x.db"]).unwrap(),
            Command::ImportFocuses {
                source: CatalogSource::Json,
                json_only: false,
                database_url: Some("x.db".to_string()),
            }
        );
        assert_eq!(
            parse(&["import-focuses", "--json-only"]).unwrap(),
            Command::ImportFocuses {
                source: CatalogSource::WikiPage,
                json_only: true,
                database_url: None,
            }
        );
    }

    #[test]
    fn the_technology_import_takes_the_same_flags() {
        assert_eq!(
            parse(&["import-technologies"]).unwrap(),
            Command::ImportTechnologies {
                source: CatalogSource::WikiPage,
                json_only: false,
                database_url: None,
            }
        );
        assert_eq!(
            parse(&["import-technologies", "--from-json", "--json-only"]).unwrap(),
            Command::ImportTechnologies {
                source: CatalogSource::Json,
                json_only: true,
                database_url: None,
            }
        );
    }

    #[test]
    fn the_staff_import_takes_the_same_flags() {
        assert_eq!(
            parse(&["import-staff", "--from-json", "--database-url", "x.db"]).unwrap(),
            Command::ImportStaff {
                source: CatalogSource::Json,
                json_only: false,
                database_url: Some("x.db".to_string()),
            }
        );
    }

    #[test]
    fn what_makes_no_sense_gets_the_usage() {
        for args in [
            &["nonsense"][..],
            &["serve", "--now"],
            &["import-focuses", "--env"],
            &["import-focuses", "--database-url"],
            &["import-technologies", "--env"],
            &["import-staff", "--env"],
        ] {
            assert!(matches!(parse(args), Err(AppError::Usage(_))));
        }
    }
}
