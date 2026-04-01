use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use notch::client::{Client, NotchError};

fn test_client(base_url: &str) -> Client {
    Client::with_token("test-token".into(), format!("{base_url}/v1")).unwrap()
}

#[tokio::test]
async fn test_fetch_markdown_success() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/test-page-id/markdown"))
        .and(header("Notion-Version", "2026-03-11"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "markdown": "Hello world",
            "truncated": false,
            "unknown_block_ids": []
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.fetch_markdown("test-page-id").await.unwrap();

    assert_eq!(resp.markdown, "Hello world");
    assert!(!resp.truncated);
}

#[tokio::test]
async fn test_fetch_markdown_truncated() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/test-page-id/markdown"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "markdown": "partial content...",
            "truncated": true,
            "unknown_block_ids": ["block-1"]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.fetch_markdown("test-page-id").await.unwrap();

    assert!(resp.truncated);
    assert_eq!(resp.unknown_block_ids, vec!["block-1"]);
}

#[tokio::test]
async fn test_fetch_metadata_title_extraction() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/test-page-id"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "test-page-id",
            "properties": {
                "Title": {
                    "type": "title",
                    "title": [
                        { "plain_text": "My " },
                        { "plain_text": "Page" }
                    ]
                }
            }
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let meta = client.fetch_metadata("test-page-id").await.unwrap();

    assert_eq!(meta.properties.title_text(), "My Page");
}

#[tokio::test]
async fn test_fetch_404_returns_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/missing-id/markdown"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "code": "object_not_found",
            "message": "Could not find page"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.fetch_markdown("missing-id").await.unwrap_err();

    assert!(matches!(err, NotchError::NotFoundOrForbidden));
}

#[tokio::test]
async fn test_fetch_403_returns_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/forbidden-id/markdown"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "status": 403,
            "code": "restricted_resource",
            "message": "Forbidden"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.fetch_markdown("forbidden-id").await.unwrap_err();

    assert!(matches!(err, NotchError::NotFoundOrForbidden));
}

#[tokio::test]
async fn test_fetch_429_returns_rate_limited() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/any-id/markdown"))
        .respond_with(
            ResponseTemplate::new(429)
                .append_header("retry-after", "0")
                .set_body_json(serde_json::json!({
                    "status": 429,
                    "code": "rate_limited",
                    "message": "Rate limited"
                })),
        )
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.fetch_markdown("any-id").await.unwrap_err();

    assert!(matches!(err, NotchError::RateLimited));
}

#[tokio::test]
async fn test_search_returns_pages() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "page-1",
                    "properties": {
                        "Name": {
                            "type": "title",
                            "title": [{ "plain_text": "Found Page" }]
                        }
                    },
                    "last_edited_time": "2026-03-13T10:00:00.000Z",
                    "url": "https://www.notion.so/Found-Page-abc123"
                }
            ],
            "has_more": false,
            "next_cursor": null
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.search("Found").await.unwrap();

    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].properties.title_text(), "Found Page");
    assert_eq!(resp.results[0].last_edited_time, "2026-03-13T10:00:00.000Z");
}

#[tokio::test]
async fn test_search_sends_page_filter() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .and(body_partial_json(serde_json::json!({
            "filter": {
                "value": "page",
                "property": "object"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [],
            "has_more": false,
            "next_cursor": null
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.search("anything").await.unwrap();
    assert!(resp.results.is_empty());
}

#[tokio::test]
async fn test_fetch_500_returns_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/error-id/markdown"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "status": 500,
            "code": "internal_server_error",
            "message": "Internal server error"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.fetch_markdown("error-id").await.unwrap_err();

    assert!(matches!(err, NotchError::Api { status: 500, .. }));
}

#[tokio::test]
async fn test_fetch_502_html_body_fallback() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/bad-gateway/markdown"))
        .respond_with(ResponseTemplate::new(502).set_body_string("<html>Bad Gateway</html>"))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.fetch_markdown("bad-gateway").await.unwrap_err();

    match err {
        NotchError::Api { status, message } => {
            assert_eq!(status, 502);
            assert_eq!(message, "HTTP 502");
        }
        other => panic!("expected NotchError::Api, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_fetch_200_malformed_json() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/broken/markdown"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all"))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.fetch_markdown("broken").await.unwrap_err();

    assert!(matches!(err, NotchError::Http(_)));
}

#[tokio::test]
async fn test_search_error_returns_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/search"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "status": 400,
            "code": "validation_error",
            "message": "Invalid query"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.search("test").await.unwrap_err();

    assert!(matches!(err, NotchError::Api { status: 400, .. }));
}

#[tokio::test]
async fn test_fetch_metadata_404_returns_not_found() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/pages/missing-id"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "code": "object_not_found",
            "message": "Not found"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.fetch_metadata("missing-id").await.unwrap_err();

    assert!(matches!(err, NotchError::NotFoundOrForbidden));
}

#[tokio::test]
async fn test_fetch_retries_on_429_then_succeeds() {
    let server = MockServer::start().await;

    // Lower priority: 200 success (catches all after 429 exhausted)
    Mock::given(method("GET"))
        .and(path("/v1/pages/retry-id/markdown"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "markdown": "retry success",
            "truncated": false,
            "unknown_block_ids": []
        })))
        .mount(&server)
        .await;

    // Higher priority: 429 once
    Mock::given(method("GET"))
        .and(path("/v1/pages/retry-id/markdown"))
        .respond_with(ResponseTemplate::new(429).append_header("retry-after", "0"))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.fetch_markdown("retry-id").await.unwrap();

    assert_eq!(resp.markdown, "retry success");
}

// T-001: FR-002 — retrieve_database が data_source_id を返す
#[tokio::test]
async fn test_retrieve_database_returns_data_sources() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/databases/db-id"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data_sources": [
                {"id": "ds-123"}
            ]
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.retrieve_database("db-id").await.unwrap();

    assert_eq!(resp.data_sources.len(), 1);
    assert_eq!(resp.data_sources[0].id, "ds-123");
}

// T-002: FR-002 — retrieve_database 404
#[tokio::test]
async fn test_retrieve_database_404() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/databases/missing-db"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "status": 404,
            "code": "object_not_found",
            "message": "Not found"
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let err = client.retrieve_database("missing-db").await.unwrap_err();

    assert!(matches!(err, NotchError::NotFoundOrForbidden));
}

// T-003: FR-003 — query_data_source が行を返す
#[tokio::test]
async fn test_query_data_source_returns_rows() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/data_sources/ds-123/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "page-1",
                    "properties": {
                        "Name": {
                            "type": "title",
                            "title": [{"plain_text": "Task A"}]
                        }
                    }
                }
            ],
            "has_more": false,
            "next_cursor": null
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.query_data_source("ds-123").await.unwrap();

    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].id, "page-1");
    assert_eq!(resp.results[0].properties.property_text("Name"), "Task A");
}

// T-004: FR-003 — query_data_source 空結果
#[tokio::test]
async fn test_query_data_source_empty_results() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/data_sources/ds-empty/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [],
            "has_more": false,
            "next_cursor": null
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());
    let resp = client.query_data_source("ds-empty").await.unwrap();

    assert!(resp.results.is_empty());
}

// T-025: FR-001 — E2E: retrieve DB → query data source → TSV 検証
#[tokio::test]
async fn test_query_e2e_retrieve_then_query() {
    let server = MockServer::start().await;

    // Step 1: retrieve_database returns data_source_id
    Mock::given(method("GET"))
        .and(path("/v1/databases/e2e-db"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data_sources": [{"id": "e2e-ds"}]
        })))
        .mount(&server)
        .await;

    // Step 2: query_data_source returns rows
    Mock::given(method("POST"))
        .and(path("/v1/data_sources/e2e-ds/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "row-1",
                    "properties": {
                        "Name": {"type": "title", "title": [{"plain_text": "Item 1"}]},
                        "Status": {"type": "select", "select": {"name": "Active"}}
                    }
                },
                {
                    "id": "row-2",
                    "properties": {
                        "Name": {"type": "title", "title": [{"plain_text": "Item 2"}]},
                        "Status": {"type": "select", "select": {"name": "Done"}}
                    }
                }
            ],
            "has_more": false,
            "next_cursor": null
        })))
        .mount(&server)
        .await;

    let client = test_client(&server.uri());

    // E2E flow
    let db = client.retrieve_database("e2e-db").await.unwrap();
    assert_eq!(db.data_sources[0].id, "e2e-ds");

    let resp = client
        .query_data_source(&db.data_sources[0].id)
        .await
        .unwrap();
    assert_eq!(resp.results.len(), 2);

    // Verify TSV output structure
    let columns = resp.results[0].properties.sorted_names();
    assert_eq!(columns, vec!["Name", "Status"]);

    let header = format!("id\t{}", columns.join("\t"));
    assert_eq!(header, "id\tName\tStatus");

    let row1_values: Vec<String> = columns
        .iter()
        .map(|col| resp.results[0].properties.property_text(col))
        .collect();
    assert_eq!(
        format!("{}\t{}", resp.results[0].id, row1_values.join("\t")),
        "row-1\tItem 1\tActive"
    );

    let row2_values: Vec<String> = columns
        .iter()
        .map(|col| resp.results[1].properties.property_text(col))
        .collect();
    assert_eq!(
        format!("{}\t{}", resp.results[1].id, row2_values.join("\t")),
        "row-2\tItem 2\tDone"
    );
}
