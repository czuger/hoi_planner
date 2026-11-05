//! Runtime settings read from the environment.

use std::path::{Path, PathBuf};

const DEFAULT_DATABASE_URL: &str = "sqlite://data/hoi4_planner.db";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:5000";

/// Server settings.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// SQLite URL or bare path (`DATABASE_URL`), the file is created if missing.
    pub database_url: String,
    /// TCP address to listen on when no socket is set (`BIND_ADDR`).
    pub bind_addr: String,
    /// Unix socket to listen on instead of TCP (`SOCKET_PATH`).
    pub socket_path: Option<PathBuf>,
    /// Directory holding `static/` and `data/` (`ASSET_ROOT`).
    pub asset_root: PathBuf,
    /// Sub-path a reverse proxy serves the app under (`URL_PREFIX`), normalized.
    pub url_prefix: String,
}

impl AppConfig {
    /// Reads the settings from environment variables, falling back to local defaults.
    pub fn from_env() -> AppConfig {
        let var =
            |name: &str, default: &str| std::env::var(name).unwrap_or_else(|_| default.to_string());
        AppConfig {
            database_url: var("DATABASE_URL", DEFAULT_DATABASE_URL),
            bind_addr: var("BIND_ADDR", DEFAULT_BIND_ADDR),
            socket_path: std::env::var_os("SOCKET_PATH")
                .filter(|path| !path.is_empty())
                .map(PathBuf::from),
            asset_root: PathBuf::from(var("ASSET_ROOT", ".")),
            url_prefix: normalize_prefix(&var("URL_PREFIX", "")),
        }
    }
}

/// `hoi4`, `/hoi4` and `/hoi4/` all mean `/hoi4`; nothing at all means the site root.
pub fn normalize_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim().trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("/{trimmed}")
    }
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
    fn the_prefix_is_normalized() {
        for prefix in ["hoi4", "/hoi4", "/hoi4/", " /hoi4/ "] {
            assert_eq!(normalize_prefix(prefix), "/hoi4");
        }
    }

    #[test]
    fn no_prefix_is_the_site_root() {
        assert_eq!(normalize_prefix(""), "");
        assert_eq!(normalize_prefix("/"), "");
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
