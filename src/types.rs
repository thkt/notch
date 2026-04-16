use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PageMarkdownResponse {
    pub markdown: String,
    pub truncated: bool,
    #[serde(default)]
    pub unknown_block_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PageMetadata {
    pub id: String,
    pub properties: PageProperties,
}

/// Notion properties map — title property is identified by `"type": "title"`, not key name.
#[derive(Debug, Deserialize)]
pub struct PageProperties(HashMap<String, serde_json::Value>);

impl PageProperties {
    pub fn title_text(&self) -> String {
        self.0
            .iter()
            .find(|(_, v)| v.get("type").and_then(|t| t.as_str()) == Some("title"))
            .map(|(key, _)| self.property_text(key))
            .unwrap_or_default()
    }

    pub fn property_text(&self, key: &str) -> String {
        let Some(value) = self.0.get(key) else {
            return String::new();
        };
        let Some(prop_type) = value.get("type").and_then(|t| t.as_str()) else {
            return String::new();
        };
        let raw = match prop_type {
            "title" => extract_rich_text_array(value, "title"),
            "rich_text" => extract_rich_text_array(value, "rich_text"),
            "number" => value
                .get("number")
                .and_then(|n| if n.is_null() { None } else { n.as_f64() })
                .map(format_number)
                .unwrap_or_default(),
            "select" | "status" => value
                .get(prop_type)
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_owned(),
            "multi_select" => value
                .get("multi_select")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
            "date" => {
                let date = value.get("date");
                let start = date
                    .and_then(|d| d.get("start"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();
                let end = date.and_then(|d| d.get("end")).and_then(|e| e.as_str());
                match end {
                    Some(e) => format!("{start} → {e}"),
                    None => start.to_owned(),
                }
            }
            "checkbox" => value
                .get("checkbox")
                .and_then(serde_json::Value::as_bool)
                .map(|b| b.to_string())
                .unwrap_or_default(),
            "url" => extract_string_field(value, "url"),
            "email" => extract_string_field(value, "email"),
            "phone_number" => extract_string_field(value, "phone_number"),
            _ => String::new(),
        };
        sanitize_value(&raw)
    }

    pub fn sorted_names(&self) -> Vec<String> {
        let mut title_key = None;
        let mut others: Vec<&str> = Vec::new();
        for (key, value) in &self.0 {
            if value.get("type").and_then(|t| t.as_str()) == Some("title") {
                title_key = Some(key.as_str());
            } else {
                others.push(key);
            }
        }
        others.sort_unstable();
        let mut result = Vec::with_capacity(self.0.len());
        if let Some(title) = title_key {
            result.push(title.to_owned());
        }
        for key in others {
            result.push(key.to_owned());
        }
        result
    }
}

fn extract_rich_text_array(value: &serde_json::Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|rt| rt.get("plain_text").and_then(|t| t.as_str()))
                .collect()
        })
        .unwrap_or_default()
}

fn extract_string_field(value: &serde_json::Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .to_owned()
}

fn sanitize_value(s: &str) -> String {
    if s.contains(['\t', '\n', '\r']) {
        s.replace(['\t', '\n', '\r'], " ")
    } else {
        s.to_owned()
    }
}

// Range-checked above: `(i64::MIN as f64..=i64::MAX as f64).contains(&n)` ensures n fits in i64
#[allow(clippy::cast_possible_truncation)]
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && (i64::MIN as f64..=i64::MAX as f64).contains(&n) {
        (n as i64).to_string()
    } else {
        n.to_string()
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub properties: PageProperties,
    pub last_edited_time: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseResponse {
    pub data_sources: Vec<DataSourceInfo>,
}

#[derive(Debug, Deserialize)]
pub struct DataSourceInfo {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct DataSourceQueryResponse {
    pub results: Vec<QueryResult>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QueryResult {
    pub id: String,
    pub properties: PageProperties,
}

#[derive(Debug, Deserialize)]
pub struct NotionErrorResponse {
    pub status: u16,
    pub code: String,
    pub message: String,
}

#[cfg(test)]
mod tests;
