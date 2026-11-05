use axum::body::Bytes;
use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use serde_json::{Map, Value};

/// A posted body as JSON, `Null` when it is not any: not declared as JSON, or unreadable.
///
/// A route says for itself what that means, rather than an extractor answering for it.
pub fn posted_json(headers: &HeaderMap, body: &Bytes) -> Value {
    let declared = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|mime| {
            let mime = mime.trim();
            mime == "application/json"
                || (mime.starts_with("application/") && mime.ends_with("+json"))
        });
    if !declared {
        return Value::Null;
    }
    serde_json::from_slice(body).unwrap_or(Value::Null)
}

/// The fields of a posted plan: a body that is no JSON object simply holds none.
pub fn posted_object(headers: &HeaderMap, body: &Bytes) -> Value {
    match posted_json(headers, body) {
        object @ Value::Object(_) => object,
        _ => Value::Object(Map::new()),
    }
}
