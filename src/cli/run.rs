use std::io::IsTerminal;
use std::path::Path;
use std::process::ExitCode;

use crate::cli::Command;
use crate::cli::healthcheck::healthcheck;
use crate::cli::serve::serve;
use crate::config::{AppConfig, DataFiles};
use crate::error::AppError;
use crate::service::{parse_focuses, parse_politics, parse_technologies};

/// Runs what the command line asks, and turns a failure into a message and an exit code.
pub async fn run(args: impl IntoIterator<Item = String>) -> ExitCode {
    tracing_subscriber::fmt()
        .with_ansi(colors_wanted())
        .with_max_level(tracing::Level::INFO)
        .init();
    match dispatch(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Colors only on a terminal, so the service's journal stays plain.
fn colors_wanted() -> bool {
    std::io::stdout().is_terminal()
}

async fn dispatch(args: impl IntoIterator<Item = String>) -> Result<(), AppError> {
    let command = Command::parse(args)?;
    let config = AppConfig::from_env();
    // The saved pages and their JSON are a laptop job, read from the checkout.
    let files = DataFiles::under(Path::new("."));
    match command {
        Command::Serve => serve(&config).await,
        Command::Healthcheck => healthcheck(&config).await,
        Command::ParseFocuses => {
            let count = parse_focuses(&files)?;
            println!("{count} focus -> {}", files.focuses_json.display());
            Ok(())
        }
        Command::ParseTechnologies => {
            let count = parse_technologies(&files)?;
            println!(
                "{count} technologies -> {}",
                files.technologies_json.display()
            );
            Ok(())
        }
        Command::ParsePolitics => run_parse_politics(&files),
    }
}

fn run_parse_politics(files: &DataFiles) -> Result<(), AppError> {
    let counts = parse_politics(files)?;
    println!(
        "{} dirigeants -> {}",
        counts.leaders,
        files.leaders_json.display()
    );
    println!(
        "{} membres de l'état-major -> {}",
        counts.staff,
        files.staff_json.display()
    );
    Ok(())
}
