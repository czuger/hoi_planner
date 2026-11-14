//! A database in a temporary folder, removed with it.

use std::ops::Deref;

use hoi4_planner::db::Store;
use tempfile::TempDir;

pub struct TempStore {
    store: Store,
    _folder: TempDir,
}

impl TempStore {
    pub fn new() -> TempStore {
        let folder = tempfile::tempdir().unwrap();
        let store = Store::open(&folder.path().join("plans.polodb")).unwrap();
        TempStore {
            store,
            _folder: folder,
        }
    }
}

impl Deref for TempStore {
    type Target = Store;

    fn deref(&self) -> &Store {
        &self.store
    }
}
