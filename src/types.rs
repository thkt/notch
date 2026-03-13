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
            .values()
            .find(|v| v.get("type").and_then(|t| t.as_str()) == Some("title"))
            .and_then(|v| v.get("title"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|rt| rt.get("plain_text").and_then(|t| t.as_str()))
                    .collect()
            })
            .unwrap_or_default()
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
pub struct NotionErrorResponse {
    pub status: u16,
    pub code: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_text_empty_properties() {
        let props: PageProperties = serde_json::from_str("{}").unwrap();
        assert_eq!(props.title_text(), "");
    }

    #[test]
    fn test_title_text_no_title_type() {
        let props: PageProperties =
            serde_json::from_str(r#"{"Tags": {"type": "multi_select"}}"#).unwrap();
        assert_eq!(props.title_text(), "");
    }

    #[test]
    fn test_title_text_with_custom_name() {
        let json = r#"{"Name": {"type": "title", "title": [{"plain_text": "My Page"}]}}"#;
        let props: PageProperties = serde_json::from_str(json).unwrap();
        assert_eq!(props.title_text(), "My Page");
    }

    #[test]
    fn test_title_text_multi_segment() {
        let json = r#"{"Title": {"type": "title", "title": [{"plain_text": "Hello "}, {"plain_text": "World"}]}}"#;
        let props: PageProperties = serde_json::from_str(json).unwrap();
        assert_eq!(props.title_text(), "Hello World");
    }
}
