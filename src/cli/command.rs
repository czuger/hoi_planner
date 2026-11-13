use crate::error::AppError;

const USAGE: &str = "Usage : hoi4_planner [serve | healthcheck | parse-focuses | parse-technologies | parse-politics]";

/// What one run of the binary is asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Serves the app, which is what no argument at all means.
    Serve,
    /// Asks a running server for `/health`.
    Healthcheck,
    /// Reads `data/focuses.html` into `data/focuses.json`.
    ParseFocuses,
    /// Reads `data/technologies/*.html` into `data/technologies.json`.
    ParseTechnologies,
    /// Reads `data/politics.html` into its two JSON files, the staff catalog among them.
    ParsePolitics,
}

impl Command {
    /// Reads the command line, without the program's name.
    ///
    /// # Errors
    /// Returns the usage when the arguments make no sense.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, AppError> {
        let args: Vec<String> = args.into_iter().collect();
        let [name] = args.as_slice() else {
            return if args.is_empty() {
                Ok(Command::Serve)
            } else {
                Err(usage())
            };
        };
        match name.as_str() {
            "serve" => Ok(Command::Serve),
            "healthcheck" => Ok(Command::Healthcheck),
            "parse-focuses" => Ok(Command::ParseFocuses),
            "parse-technologies" => Ok(Command::ParseTechnologies),
            "parse-politics" => Ok(Command::ParsePolitics),
            _ => Err(usage()),
        }
    }
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
    fn each_page_has_its_command() {
        assert_eq!(parse(&["parse-focuses"]).unwrap(), Command::ParseFocuses);
        assert_eq!(
            parse(&["parse-technologies"]).unwrap(),
            Command::ParseTechnologies
        );
        assert_eq!(parse(&["parse-politics"]).unwrap(), Command::ParsePolitics);
    }

    #[test]
    fn what_makes_no_sense_gets_the_usage() {
        for args in [
            &["nonsense"][..],
            &["serve", "--now"],
            &["seed"],
            &["import-focuses"],
            &["parse-focuses", "--json-only"],
        ] {
            assert!(matches!(parse(args), Err(AppError::Usage(_))));
        }
    }
}
