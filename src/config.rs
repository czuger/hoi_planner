//! Runtime settings: fixed, except the socket and data folder `cargo deploy` hands over.

use std::path::{Path, PathBuf};

/// The variable naming the folder of everything the app keeps (`cargo deploy`'s `<PREFIX>_DATA_DIR`).
pub const DATA_DIR_VAR: &str = "HOI4_PLANNER_DATA_DIR";
/// The variable naming the Unix socket to listen on (`cargo deploy`'s `<PREFIX>_SOCKET`).
pub const SOCKET_VAR: &str = "HOI4_PLANNER_SOCKET";

const DEFAULT_DATA_DIR: &str = "data";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:5000";
const DATABASE_FILE: &str = "hoi4_planner.db";

/// Server settings.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// The SQLite file, in `data/` or the data folder `cargo deploy` hands over; created if missing.
    pub database_url: String,
    /// TCP address to listen on when no socket is set.
    pub bind_addr: String,
    /// Unix socket to listen on instead of TCP, handed over by `cargo deploy`.
    pub socket_path: Option<PathBuf>,
}

impl AppConfig {
    /// The local defaults, with the socket and data folder `cargo deploy` sets, if any.
    pub fn from_env() -> AppConfig {
        let set = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        let data_dir =
            set(DATA_DIR_VAR).map_or_else(|| PathBuf::from(DEFAULT_DATA_DIR), PathBuf::from);
        AppConfig {
            database_url: database_path(&data_dir),
            bind_addr: DEFAULT_BIND_ADDR.to_string(),
            socket_path: set(SOCKET_VAR).map(PathBuf::from),
        }
    }
}

/// The database file of a data folder, as the bare path SQLite accepts for a URL.
pub fn database_path(data_dir: &Path) -> String {
    data_dir.join(DATABASE_FILE).display().to_string()
}

/// What a database is called on screen: its file name, whatever the URL around it.
pub fn database_label(database_url: &str) -> String {
    let path = database_url.split('?').next().unwrap_or(database_url);
    if path.contains(":memory:") {
        return ":memory:".to_string();
    }
    Path::new(path.trim_start_matches("sqlite:"))
        .file_name()
        .map_or_else(
            || database_url.to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
}

/// One saved page of the research tree.
#[derive(Debug, Clone)]
pub struct TechnologyPageFile {
    /// The research area it covers, as the catalog names it on screen.
    pub domain: &'static str,
    /// The file.
    pub path: PathBuf,
}

/// The saved research pages, DLC versions only, in the order the catalog lists them.
const TECHNOLOGY_PAGES: [(&str, &str); 9] = [
    ("Infanterie", "Infantry_technology.html"),
    ("Compagnies de soutien", "Support_Companies_technology.html"),
    ("Blindés", "Armor_technology.html"),
    ("Artillerie", "Artillery_technology.html"),
    ("Ingénierie", "Engineering_technology.html"),
    ("Industrie", "Industry_technology.html"),
    ("Aviation", "Air_technology.html"),
    ("Marine", "Naval_technology.html"),
    ("Soutien naval", "Naval_support_technology.html"),
];

/// The files the wiki pages are read from and their parsed catalogs are written to.
#[derive(Debug, Clone)]
pub struct DataFiles {
    /// The German national focus tree page, saved by hand.
    pub wiki_page: PathBuf,
    /// The catalog parsed out of it, committed, which is what a database is seeded from.
    pub focuses_json: PathBuf,
    /// The German Reich country page, saved by hand.
    pub politics_page: PathBuf,
    /// The country leaders parsed out of it.
    pub leaders_json: PathBuf,
    /// The staff and designers parsed out of it.
    pub staff_json: PathBuf,
    /// The research pages, saved by hand.
    pub technology_pages: Vec<TechnologyPageFile>,
    /// The research tree parsed out of them, committed, which a database is seeded from.
    pub technologies_json: PathBuf,
}

impl DataFiles {
    /// The files of the `data/` folder under `root`.
    pub fn under(root: &Path) -> DataFiles {
        let data = root.join("data");
        DataFiles {
            wiki_page: data.join("focuses.html"),
            focuses_json: data.join("focuses.json"),
            politics_page: data.join("politics.html"),
            leaders_json: data.join("politics.json"),
            staff_json: data.join("staff.json"),
            technology_pages: TECHNOLOGY_PAGES
                .iter()
                .map(|(domain, file)| TechnologyPageFile {
                    domain,
                    path: data.join("technologies").join(file),
                })
                .collect(),
            technologies_json: data.join("technologies.json"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_database_lives_in_the_data_folder() {
        assert_eq!(
            database_path(Path::new("/srv/app/data")),
            "/srv/app/data/hoi4_planner.db"
        );
        assert_eq!(database_path(Path::new("data")), "data/hoi4_planner.db");
    }

    #[test]
    fn a_database_is_labelled_by_its_file_name() {
        assert_eq!(
            database_label("sqlite://data/hoi4_planner.db"),
            "hoi4_planner.db"
        );
        assert_eq!(database_label("/srv/app/plans.db"), "plans.db");
        assert_eq!(database_label("sqlite::memory:"), ":memory:");
    }
}
