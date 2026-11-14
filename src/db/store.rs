use std::path::Path;
use std::sync::{Arc, PoisonError, RwLock};

use polodb_core::{CollectionT, Database};

use crate::error::AppError;

/// The collection holding the plans, the draft included.
pub(crate) const PLANS: &str = "plans";

/// The PoloDB database, shared by every request.
///
/// PoloDB answers synchronously, so each call runs on Tokio's blocking pool. The database can be
/// closed, after which every call fails as a database that cannot be read does.
#[derive(Clone)]
pub struct Store {
    database: Arc<RwLock<Option<Database>>>,
}

impl Store {
    /// Opens the database folder, creating it and its parents if needed.
    ///
    /// # Errors
    /// Fails when the folder cannot be created or the database cannot be opened, as when
    /// another process holds it.
    pub fn open(path: &Path) -> Result<Store, AppError> {
        std::fs::create_dir_all(path).map_err(AppError::DatabaseFolder)?;
        let database = Database::open_path(path)?;
        Ok(Store {
            database: Arc::new(RwLock::new(Some(database))),
        })
    }

    /// Lets go of the database: every later call fails.
    pub fn close(&self) {
        let mut database = self
            .database
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        *database = None;
    }

    /// Asks the database a trivial question, to know whether it answers.
    ///
    /// # Errors
    /// Fails when the database cannot be read.
    pub async fn ping(&self) -> Result<(), AppError> {
        self.run(|database| {
            database.collection::<()>(PLANS).count_documents()?;
            Ok(())
        })
        .await
    }

    /// Runs `work` on the database, off the async runtime.
    pub(crate) async fn run<T, F>(&self, work: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&Database) -> Result<T, AppError> + Send + 'static,
    {
        let database = self
            .database
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .ok_or(AppError::DatabaseClosed)?;
        tokio::task::spawn_blocking(move || work(&database)).await?
    }
}
