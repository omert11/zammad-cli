use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::{Map, Value};
use std::path::Path;

/// Split a comma-separated CSV string into trimmed, non-empty tokens.
pub fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// Read each file path and build Zammad article `attachments[]` entries
/// (`filename`, `data` (base64), `mime-type`). Async to avoid blocking the tokio worker.
pub async fn build_attachments(paths: &[String]) -> Result<Vec<Value>> {
    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        let bytes = tokio::fs::read(p)
            .await
            .with_context(|| format!("Failed to read attachment: {p}"))?;
        let path = Path::new(p);
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .essence_str()
            .to_string();
        out.push(serde_json::json!({
            "filename": filename,
            "data": B64.encode(&bytes),
            "mime-type": mime,
        }));
    }
    Ok(out)
}

/// Parse a comma-separated attachment-path string and read them into JSON entries.
/// Returns an empty vec when `csv` is `None` or empty.
pub async fn build_attachments_opt(csv: Option<&str>) -> Result<Vec<Value>> {
    let paths = csv.map(split_csv).unwrap_or_default();
    if paths.is_empty() {
        Ok(Vec::new())
    } else {
        build_attachments(&paths).await
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}...")
    }
}

pub fn insert_opt_str(body: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(v) = value {
        body.insert(key.to_string(), Value::String(v));
    }
}

/// Build Zammad search query from named filter parts (`field`, `value`).
/// Auto-quotes values containing whitespace.
pub fn build_search_query<S: AsRef<str>>(parts: &[(&str, S)]) -> String {
    if parts.is_empty() {
        return "*".to_string();
    }
    parts
        .iter()
        .map(|(field, value)| {
            let v = value.as_ref();
            if v.contains(char::is_whitespace) {
                format!(r#"{field}:"{v}""#)
            } else {
                format!("{field}:{v}")
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}
