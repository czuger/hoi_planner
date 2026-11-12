use std::io::IsTerminal;
use std::process::ExitCode;

use tracing_subscriber::EnvFilter;

use crate::cli::Command;
use crate::cli::healthcheck::healthcheck;
use crate::cli::serve::serve;
use crate::config::{AppConfig, DataFiles, database_label};
use crate::error::AppError;
use crate::service::{
    CatalogSource, ImportOptions, import_focuses, import_staff, import_technologies, parse_politics,
};

/// Runs what the command line asks, and turns a failure into a message and an exit code.
pub async fn run(args: impl IntoIterator<Item = String>) -> ExitCode {
    tracing_subscriber::fmt()
        .with_ansi(std::io::stdout().is_terminal())
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    match dispatch(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn dispatch(args: impl IntoIterator<Item = String>) -> Result<(), AppError> {
    let command = Command::parse(args)?;
    let config = AppConfig::from_env();
    let files = DataFiles::under(&config.asset_root);
    match command {
        Command::Serve => serve(&config).await,
        Command::Healthcheck => healthcheck(&config).await,
        Command::ParsePolitics => run_parse_politics(&files),
        Command::ImportFocuses {
            source,
            json_only,
            database_url,
        } => {
            let url = database_url.unwrap_or(config.database_url);
            run_import(&files, source, (!json_only).then_some(url.as_str())).await
        }
        Command::ImportTechnologies {
            source,
            json_only,
            database_url,
        } => {
            let url = database_url.unwrap_or(config.database_url);
            run_import_technologies(&files, source, (!json_only).then_some(url.as_str())).await
        }
        Command::ImportStaff {
            source,
            json_only,
            database_url,
        } => {
            let url = database_url.unwrap_or(config.database_url);
            run_import_staff(&files, source, (!json_only).then_some(url.as_str())).await
        }
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

async fn run_import(
    files: &DataFiles,
    source: CatalogSource,
    database_url: Option<&str>,
) -> Result<(), AppError> {
    if let Some(url) = database_url {
        println!("remplissage de {}", database_label(url));
    }
    let outcome = import_focuses(
        files,
        ImportOptions {
            source,
            database_url,
        },
    )
    .await?;
    println!("{} focus lus", outcome.parsed);
    if source == CatalogSource::WikiPage {
        println!("{} écrit", files.focuses_json.display());
    }
    if let (Some(stored), Some(url)) = (outcome.stored, database_url) {
        println!("{stored} focus enregistrés dans {}", database_label(url));
    }
    Ok(())
}

async fn run_import_technologies(
    files: &DataFiles,
    source: CatalogSource,
    database_url: Option<&str>,
) -> Result<(), AppError> {
    if let Some(url) = database_url {
        println!("remplissage de {}", database_label(url));
    }
    let outcome = import_technologies(
        files,
        ImportOptions {
            source,
            database_url,
        },
    )
    .await?;
    println!("{} technologies lues", outcome.parsed);
    if source == CatalogSource::WikiPage {
        println!("{} écrit", files.technologies_json.display());
    }
    if let (Some(stored), Some(url)) = (outcome.stored, database_url) {
        println!(
            "{stored} technologies enregistrées dans {}",
            database_label(url)
        );
    }
    Ok(())
}

async fn run_import_staff(
    files: &DataFiles,
    source: CatalogSource,
    database_url: Option<&str>,
) -> Result<(), AppError> {
    if let Some(url) = database_url {
        println!("remplissage de {}", database_label(url));
    }
    let outcome = import_staff(
        files,
        ImportOptions {
            source,
            database_url,
        },
    )
    .await?;
    println!("{} membres de l'état-major lus", outcome.parsed);
    if source == CatalogSource::WikiPage {
        println!("{} écrit", files.staff_json.display());
    }
    if let (Some(stored), Some(url)) = (outcome.stored, database_url) {
        println!(
            "{stored} membres de l'état-major enregistrés dans {}",
            database_label(url)
        );
    }
    Ok(())
}
