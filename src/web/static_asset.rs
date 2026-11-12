//! The page's stylesheet and scripts, compiled into the binary: a release is that one file.

use axum::extract::Path;
use axum::http::header::{CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

const CSS: &str = "text/css; charset=utf-8";
const JAVASCRIPT: &str = "text/javascript; charset=utf-8";

/// One file of `static/`, as served under `/static/`.
#[derive(Debug)]
pub struct StaticAsset {
    /// Its file name.
    pub name: &'static str,
    /// Its media type.
    pub content_type: &'static str,
    /// Its text.
    pub body: &'static str,
    /// A hash of its text, which changes with each edit: browsers revalidate against it.
    pub hash: u64,
}

macro_rules! asset {
    ($name:literal, $content_type:expr) => {{
        let body = include_str!(concat!("../../static/", $name));
        StaticAsset {
            name: $name,
            content_type: $content_type,
            body,
            hash: fnv1a(body.as_bytes()),
        }
    }};
}

const ASSETS: [StaticAsset; 6] = [
    asset!("app.css", CSS),
    asset!("common.js", JAVASCRIPT),
    asset!("technologies.js", JAVASCRIPT),
    asset!("staff.js", JAVASCRIPT),
    asset!("comments.js", JAVASCRIPT),
    asset!("app.js", JAVASCRIPT),
];

/// FNV-1a, computed at compile time: an `ETag` only has to change when the file does.
const fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
        index += 1;
    }
    hash
}

/// Serves one asset, or `304` when the browser's copy is still the one compiled in.
pub async fn serve_static(Path(name): Path<String>, headers: HeaderMap) -> Response {
    let Some(asset) = ASSETS.iter().find(|asset| asset.name == name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let etag = format!("\"{:016x}\"", asset.hash);
    let unchanged = headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|tag| tag.trim() == etag));
    if unchanged {
        return (StatusCode::NOT_MODIFIED, [(ETAG, etag)]).into_response();
    }
    (
        [(CONTENT_TYPE, asset.content_type.to_string()), (ETAG, etag)],
        asset.body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_file_of_the_static_folder_is_compiled_in() {
        let mut on_disk: Vec<String> =
            std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/static"))
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|name| !name.starts_with('.'))
                .collect();
        on_disk.sort();
        let mut compiled: Vec<&str> = ASSETS.iter().map(|asset| asset.name).collect();
        compiled.sort_unstable();
        assert_eq!(on_disk, compiled);
    }

    #[test]
    fn the_hash_follows_the_text() {
        assert_ne!(fnv1a(b"a"), fnv1a(b"b"));
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    }
}
