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

/// Minimal ISO 8601 date-time validation without pulling in a date crate.
///
/// Accepts `YYYY-MM-DDTHH:MM` with optional `:SS`, optional fractional seconds
/// (`.sss`), and an optional zone suffix (`Z` or `±HH:MM`). Field ranges are
/// sanity-checked (month 1-12, day 1-31, hour 0-23, minute/second 0-59) so an
/// obvious typo (`2026-13-40`, `tomorrow`) fails locally instead of surfacing
/// Zammad's opaque 422. It is a shape check, not a full calendar validator
/// (e.g. it does not reject Feb 30) — Zammad makes the final ruling.
pub fn is_iso8601_datetime(s: &str) -> bool {
    let bytes = s.as_bytes();
    // Need at least "YYYY-MM-DDTHH:MM".
    if bytes.len() < 16 {
        return false;
    }
    // ASCII-only positions are required for the fixed-offset slicing below.
    if !s.is_ascii() {
        return false;
    }
    let two = |a: usize, b: usize| s[a..b].parse::<u32>().ok();
    let in_range = |v: Option<u32>, lo: u32, hi: u32| v.is_some_and(|n| n >= lo && n <= hi);

    // Fixed separators.
    if &s[4..5] != "-" || &s[7..8] != "-" {
        return false;
    }
    let date_time_sep = &s[10..11];
    if date_time_sep != "T" && date_time_sep != " " {
        return false;
    }
    if &s[13..14] != ":" {
        return false;
    }

    if s[0..4].parse::<u32>().is_err() {
        return false; // year (any 4 digits)
    }
    if !in_range(two(5, 7), 1, 12) {
        return false; // month
    }
    if !in_range(two(8, 10), 1, 31) {
        return false; // day
    }
    if !in_range(two(11, 13), 0, 23) {
        return false; // hour
    }
    if !in_range(two(14, 16), 0, 59) {
        return false; // minute
    }

    // Optional `:SS` and everything after (fraction / zone) is left loose —
    // it covers `Z`, `+03:00`, `.000Z`, etc. — Zammad does the strict parse.
    let rest = &s[16..];
    if rest.is_empty() {
        return true;
    }
    if let Some(sec) = rest.strip_prefix(':') {
        // Seconds must be two digits 00-59; the remainder is the zone/fraction.
        if sec.len() < 2 {
            return false;
        }
        return in_range(sec[0..2].parse::<u32>().ok(), 0, 59);
    }
    // Directly a zone/fraction after HH:MM (no seconds) — accept.
    true
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_accepts_valid_forms() {
        assert!(is_iso8601_datetime("2026-06-18T17:00:00.000Z"));
        assert!(is_iso8601_datetime("2026-06-18T17:00:00Z"));
        assert!(is_iso8601_datetime("2026-06-18T17:00:00"));
        assert!(is_iso8601_datetime("2026-06-18T17:00"));
        assert!(is_iso8601_datetime("2026-06-18T17:00:00+03:00"));
        assert!(is_iso8601_datetime("2026-06-18 17:00:00Z")); // space separator
        assert!(is_iso8601_datetime("2026-12-31T23:59:59Z"));
    }

    #[test]
    fn iso8601_rejects_invalid_forms() {
        assert!(!is_iso8601_datetime("tomorrow"));
        assert!(!is_iso8601_datetime("2026-13-40")); // bad month/day, no time
        assert!(!is_iso8601_datetime("2026-13-01T10:00")); // month 13
        assert!(!is_iso8601_datetime("2026-06-32T10:00")); // day 32
        assert!(!is_iso8601_datetime("2026-06-18T24:00")); // hour 24
        assert!(!is_iso8601_datetime("2026-06-18T10:60")); // minute 60
        assert!(!is_iso8601_datetime("2026/06/18T10:00")); // wrong separators
        assert!(!is_iso8601_datetime("2026-06-18X10:00")); // wrong date/time sep
        assert!(!is_iso8601_datetime("2026-06-18T10:00:6")); // 1-digit seconds
        assert!(!is_iso8601_datetime("2026-06-18T10:00:99")); // seconds 99
        assert!(!is_iso8601_datetime("")); // empty
        assert!(!is_iso8601_datetime("2026-06-18")); // date only
    }
}
