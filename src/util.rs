use serde_json::{Map, Value};

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
