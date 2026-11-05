//! Application errors.

use thiserror::Error;

use crate::domain::{PlanFieldsError, PlanFileError};

/// Everything that can go wrong while handling a request or running a command.
///
/// The `Display` text reaches the user, so it is written in French.
#[derive(Debug, Error)]
pub enum AppError {
    /// The database failed.
    #[error("Erreur de base de données.")]
    Database(#[from] sqlx::Error),
    /// The database schema could not be migrated.
    #[error("Impossible de mettre à jour la base de données.")]
    Migration(#[from] sqlx::migrate::MigrateError),
    /// The folder holding the database file could not be created.
    #[error("Impossible de créer le dossier de la base de données.")]
    DatabaseFolder(#[source] std::io::Error),
    /// A page could not be rendered.
    #[error("Impossible d'afficher la page.")]
    Template(#[from] askama::Error),
    /// A stored value could not be read back.
    #[error("Donnée invalide en base ({column}).")]
    CorruptData {
        /// The column holding the value.
        column: &'static str,
        /// Why it could not be read.
        #[source]
        source: serde_json::Error,
    },
    /// A value could not be written as JSON.
    #[error("Impossible d'écrire le JSON.")]
    Json(#[from] serde_json::Error),
    /// A file needed by a command is missing or unreadable.
    #[error("Impossible de lire {path} : {source}")]
    FileRead {
        /// The file.
        path: String,
        /// What the system said.
        #[source]
        source: std::io::Error,
    },
    /// A file written by a command could not be written.
    #[error("Impossible d'écrire {path} : {source}")]
    FileWrite {
        /// The file.
        path: String,
        /// What the system said.
        #[source]
        source: std::io::Error,
    },
    /// A catalog file is not a list of what it should hold.
    #[error("{path} n'est pas un catalogue lisible : {source}")]
    InvalidCatalogFile {
        /// The file.
        path: String,
        /// What the parser said.
        #[source]
        source: serde_json::Error,
    },
    /// A saved page holds nothing the parser recognizes.
    #[error("{what} introuvable dans la page enregistrée : sa structure a-t-elle changé ?")]
    NothingParsed {
        /// What was looked for.
        what: &'static str,
    },
    /// The server's Unix socket or TCP address could not be used.
    #[error("Impossible d'écouter sur {address} : {source}")]
    Listen {
        /// The socket path or TCP address.
        address: String,
        /// What the system said.
        #[source]
        source: std::io::Error,
    },
    /// The server did not answer the health check.
    #[error("Le serveur ne répond pas correctement : {0}")]
    Unhealthy(String),
    /// The command line makes no sense.
    #[error("{0}")]
    Usage(String),
    /// The posted plan is not one a plan may be.
    #[error(transparent)]
    PlanFields(#[from] PlanFieldsError),
    /// The posted file is not a plan file.
    #[error(transparent)]
    PlanFile(#[from] PlanFileError),
    /// The plan has no name.
    #[error("Un plan doit avoir un nom.")]
    PlanNameMissing,
    /// The name of the plan has nothing an id could be made from.
    #[error("Ce nom ne contient ni lettre ni chiffre.")]
    PlanNameUnusable,
    /// No plan has this id.
    #[error("Ce plan n'existe pas.")]
    PlanNotFound,
}

impl AppError {
    /// Returns `true` when the database did not do what was asked.
    pub fn is_database(&self) -> bool {
        matches!(
            self,
            AppError::Database(_) | AppError::Migration(_) | AppError::CorruptData { .. }
        )
    }
}
