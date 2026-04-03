use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::json;
use thiserror::Error;

use crate::types::{
    DataSourceQueryResponse, DatabaseResponse, NotionErrorResponse, PageMarkdownResponse,
    PageMetadata, SearchResponse,
};

const NOTION_API_BASE: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2026-03-11";

#[derive(Debug, Error)]
pub enum NotchError {
    #[error(
        "NOTION_TOKEN not set. Create an integration at https://www.notion.so/profile/integrations"
    )]
    TokenNotSet,

    #[error("Page not found. Ensure the page is shared with your integration")]
    NotFoundOrForbidden,

    #[error("Rate limited by Notion API. Please wait and retry")]
    RateLimited,

    #[error("Notion API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Database has no data sources")]
    NoDataSources,

    #[error("Invalid Notion URL: {0}")]
    InvalidUrl(String),

    #[error("{0}")]
    InvalidInput(String),

    #[error("Failed to read stdin: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

#[derive(Debug)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new() -> Result<Self, NotchError> {
        let token = std::env::var("NOTION_TOKEN").map_err(|_| NotchError::TokenNotSet)?;
        Self::with_token(token, NOTION_API_BASE.to_string())
    }

    #[doc(hidden)]
    pub fn with_token(token: String, base_url: String) -> Result<Self, NotchError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|e| NotchError::Api {
                status: 0,
                message: format!("Invalid token: {e}"),
            })?,
        );
        headers.insert("Notion-Version", HeaderValue::from_static(NOTION_VERSION));

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(concat!("notch/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self { http, base_url })
    }

    pub async fn fetch_markdown(&self, page_id: &str) -> Result<PageMarkdownResponse, NotchError> {
        let url = format!("{}/pages/{}/markdown", self.base_url, page_id);
        let resp = self.send_with_retry(|| self.http.get(&url)).await?;
        self.handle_response(resp).await
    }

    pub async fn fetch_metadata(&self, page_id: &str) -> Result<PageMetadata, NotchError> {
        let url = format!("{}/pages/{}", self.base_url, page_id);
        let resp = self.send_with_retry(|| self.http.get(&url)).await?;
        self.handle_response(resp).await
    }

    pub async fn retrieve_database(&self, db_id: &str) -> Result<DatabaseResponse, NotchError> {
        let url = format!("{}/databases/{}", self.base_url, db_id);
        let resp = self.send_with_retry(|| self.http.get(&url)).await?;
        self.handle_response(resp).await
    }

    pub async fn query_data_source(
        &self,
        ds_id: &str,
    ) -> Result<DataSourceQueryResponse, NotchError> {
        let url = format!("{}/data_sources/{}/query", self.base_url, ds_id);
        let resp = self
            .send_with_retry(|| self.http.post(&url).json(&serde_json::json!({})))
            .await?;
        self.handle_response(resp).await
    }

    pub async fn search(&self, query: &str) -> Result<SearchResponse, NotchError> {
        let url = format!("{}/search", self.base_url);
        let body = json!({
            "query": query,
            "filter": {
                "value": "page",
                "property": "object"
            }
        });
        let resp = self
            .send_with_retry(|| self.http.post(&url).json(&body))
            .await?;
        self.handle_response(resp).await
    }

    async fn send_with_retry(
        &self,
        build_request: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, NotchError> {
        for attempt in 0..3u32 {
            let resp = build_request().send().await?;
            match resp.status().as_u16() {
                429 => {
                    let wait = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(1);
                    tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
                }
                500..=599 => {
                    let wait_ms = 100u64 << attempt; // 100ms, 200ms, 400ms
                    tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
                }
                _ => return Ok(resp),
            }
        }
        Ok(build_request().send().await?)
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        resp: reqwest::Response,
    ) -> Result<T, NotchError> {
        let status = resp.status().as_u16();
        if status == 200 {
            return Ok(resp.json().await?);
        }

        match status {
            403 | 404 => Err(NotchError::NotFoundOrForbidden),
            429 => Err(NotchError::RateLimited),
            _ => {
                let err: NotionErrorResponse = resp.json().await.unwrap_or(NotionErrorResponse {
                    status,
                    code: "unknown".to_string(),
                    message: format!("HTTP {status}"),
                });
                Err(NotchError::Api {
                    status: err.status,
                    message: err.message,
                })
            }
        }
    }
}

pub fn parse_page_id(input: &str) -> Result<String, NotchError> {
    if is_uuid(input) {
        return Ok(input.to_string());
    }

    if is_hex32(input) {
        return Ok(format_uuid(input));
    }

    let parsed = url::Url::parse(input).map_err(|_| NotchError::InvalidUrl(input.to_string()))?;

    let host = parsed.host_str().unwrap_or("");
    let is_notion = host == "notion.so"
        || host.ends_with(".notion.so")
        || host == "notion.site"
        || host.ends_with(".notion.site");
    if !is_notion {
        return Err(NotchError::InvalidUrl(input.to_string()));
    }

    let path = parsed.path();
    let last_segment = path.rsplit('/').next().unwrap_or("");

    if is_uuid(last_segment) {
        return Ok(last_segment.to_string());
    }

    if let Some(hex) = extract_hex32_suffix(last_segment) {
        return Ok(format_uuid(hex));
    }

    for (key, value) in parsed.query_pairs() {
        if key == "p" && is_hex32(&value) {
            return Ok(format_uuid(&value));
        }
    }

    Err(NotchError::InvalidUrl(input.to_string()))
}

fn is_uuid(s: &str) -> bool {
    if s.len() != 36 {
        return false;
    }
    let expected = [8, 4, 4, 4, 12];
    let mut parts = s.split('-');
    expected.iter().all(|&len| {
        parts
            .next()
            .is_some_and(|p| p.len() == len && p.bytes().all(|b| b.is_ascii_hexdigit()))
    }) && parts.next().is_none()
}

fn is_hex32(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn format_uuid(hex: &str) -> String {
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn extract_hex32_suffix(s: &str) -> Option<&str> {
    if s.len() >= 32 {
        let suffix = &s[s.len() - 32..];
        if suffix.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Some(suffix);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uuid_direct() {
        let id = "12345678-1234-1234-1234-123456789abc";
        assert_eq!(parse_page_id(id).unwrap(), id);
    }

    #[test]
    fn test_parse_hex32() {
        let hex = "123456781234123412341234567890ab";
        assert_eq!(
            parse_page_id(hex).unwrap(),
            "12345678-1234-1234-1234-1234567890ab"
        );
    }

    #[test]
    fn test_parse_notion_url_with_title() {
        let url = "https://www.notion.so/My-Page-Title-123456781234123412341234567890ab";
        assert_eq!(
            parse_page_id(url).unwrap(),
            "12345678-1234-1234-1234-1234567890ab"
        );
    }

    #[test]
    fn test_parse_notion_url_with_workspace() {
        let url = "https://www.notion.so/workspace/123456781234123412341234567890ab";
        assert_eq!(
            parse_page_id(url).unwrap(),
            "12345678-1234-1234-1234-1234567890ab"
        );
    }

    #[test]
    fn test_parse_notion_url_with_query_param() {
        let url = "https://www.notion.so/page?p=123456781234123412341234567890ab";
        assert_eq!(
            parse_page_id(url).unwrap(),
            "12345678-1234-1234-1234-1234567890ab"
        );
    }

    #[test]
    fn test_parse_invalid_url() {
        assert!(parse_page_id("https://example.com/page").is_err());
    }

    #[test]
    fn test_parse_spoofed_domain_rejected() {
        assert!(
            parse_page_id("https://evil-notion.so/Page-123456781234123412341234567890ab").is_err()
        );
    }

    #[test]
    fn test_parse_invalid_string() {
        assert!(parse_page_id("not-a-valid-id").is_err());
    }

    #[test]
    fn test_parse_notion_site_url() {
        let url = "https://myworkspace.notion.site/Page-123456781234123412341234567890ab";
        assert_eq!(
            parse_page_id(url).unwrap(),
            "12345678-1234-1234-1234-1234567890ab"
        );
    }

    #[test]
    fn test_parse_notion_url_without_www() {
        let url = "https://notion.so/Page-123456781234123412341234567890ab";
        assert_eq!(
            parse_page_id(url).unwrap(),
            "12345678-1234-1234-1234-1234567890ab"
        );
    }

    #[test]
    fn test_parse_notion_url_with_fragment() {
        let url = "https://www.notion.so/Page-123456781234123412341234567890ab#section";
        assert_eq!(
            parse_page_id(url).unwrap(),
            "12345678-1234-1234-1234-1234567890ab"
        );
    }

    #[test]
    fn test_parse_notion_url_with_uuid_in_path() {
        let url = "https://www.notion.so/12345678-1234-1234-1234-1234567890ab";
        assert_eq!(
            parse_page_id(url).unwrap(),
            "12345678-1234-1234-1234-1234567890ab"
        );
    }

    #[test]
    fn test_parse_empty_string() {
        assert!(parse_page_id("").is_err());
    }
}
