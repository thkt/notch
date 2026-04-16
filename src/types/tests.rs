use super::*;

// T-031: title_text — プロパティが空のとき空文字列を返す
#[test]
fn test_title_text_empty_properties() {
    let props: PageProperties = serde_json::from_str("{}").unwrap();
    assert_eq!(props.title_text(), "");
}

// T-032: title_text — title type を持つプロパティがないとき空文字列
#[test]
fn test_title_text_no_title_type() {
    let props: PageProperties =
        serde_json::from_str(r#"{"Tags": {"type": "multi_select"}}"#).unwrap();
    assert_eq!(props.title_text(), "");
}

// T-033: title_text — カスタムキー名でも title type を検出する
#[test]
fn test_title_text_with_custom_name() {
    let json = r#"{"Name": {"type": "title", "title": [{"plain_text": "My Page"}]}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.title_text(), "My Page");
}

// T-034: title_text — 複数セグメントを結合する
#[test]
fn test_title_text_multi_segment() {
    let json = r#"{"Title": {"type": "title", "title": [{"plain_text": "Hello "}, {"plain_text": "World"}]}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.title_text(), "Hello World");
}

// T-005: FR-004 — title プロパティの plain text 抽出
#[test]
fn test_property_text_title() {
    let json = r#"{"Name": {"type": "title", "title": [{"plain_text": "Hello"}]}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Name"), "Hello");
}

// T-006: FR-004 — rich_text プロパティの plain text 抽出
#[test]
fn test_property_text_rich_text() {
    let json = r#"{"Desc": {"type": "rich_text", "rich_text": [{"plain_text": "world"}]}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Desc"), "world");
}

// T-007: FR-005 — number プロパティ
#[test]
fn test_property_text_number() {
    let json = r#"{"Count": {"type": "number", "number": 42}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Count"), "42");
}

// T-008: FR-005 — number null
#[test]
fn test_property_text_number_null() {
    let json = r#"{"Count": {"type": "number", "number": null}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Count"), "");
}

// T-009: FR-006 — select プロパティ
#[test]
fn test_property_text_select() {
    let json = r#"{"Status": {"type": "select", "select": {"name": "Done"}}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Status"), "Done");
}

// T-010: FR-006 — select null
#[test]
fn test_property_text_select_null() {
    let json = r#"{"Status": {"type": "select", "select": null}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Status"), "");
}

// T-011: FR-006 — status プロパティ
#[test]
fn test_property_text_status() {
    let json = r#"{"State": {"type": "status", "status": {"name": "In Progress"}}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("State"), "In Progress");
}

// T-012: FR-007 — multi_select プロパティ
#[test]
fn test_property_text_multi_select() {
    let json =
        r#"{"Tags": {"type": "multi_select", "multi_select": [{"name": "A"}, {"name": "B"}]}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Tags"), "A, B");
}

// T-013: FR-008 — date 単一
#[test]
fn test_property_text_date() {
    let json = r#"{"Due": {"type": "date", "date": {"start": "2026-03-16", "end": null}}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Due"), "2026-03-16");
}

// T-014: FR-008 — date range
#[test]
fn test_property_text_date_range() {
    let json =
        r#"{"Period": {"type": "date", "date": {"start": "2026-03-16", "end": "2026-03-20"}}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Period"), "2026-03-16 → 2026-03-20");
}

// T-015: FR-009 — checkbox true
#[test]
fn test_property_text_checkbox_true() {
    let json = r#"{"Done": {"type": "checkbox", "checkbox": true}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Done"), "true");
}

// T-016: FR-009 — checkbox false
#[test]
fn test_property_text_checkbox_false() {
    let json = r#"{"Done": {"type": "checkbox", "checkbox": false}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Done"), "false");
}

// T-017: FR-010 — url プロパティ
#[test]
fn test_property_text_url() {
    let json = r#"{"Link": {"type": "url", "url": "https://example.com"}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Link"), "https://example.com");
}

// T-018: FR-010 — email プロパティ
#[test]
fn test_property_text_email() {
    let json = r#"{"Mail": {"type": "email", "email": "a@b.com"}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Mail"), "a@b.com");
}

// T-019: FR-010 — phone_number プロパティ
#[test]
fn test_property_text_phone() {
    let json = r#"{"Phone": {"type": "phone_number", "phone_number": "+81-90-1234-5678"}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Phone"), "+81-90-1234-5678");
}

// T-020: FR-011 — 未対応型は空文字列
#[test]
fn test_property_text_unsupported_type() {
    let json = r#"{"Calc": {"type": "formula", "formula": {"type": "string", "string": "x"}}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Calc"), "");
}

// T-035: property_text — 存在しないキーは空文字列を返す
#[test]
fn test_property_text_missing_key() {
    let props: PageProperties = serde_json::from_str("{}").unwrap();
    assert_eq!(props.property_text("Missing"), "");
}

// T-001 prep: DatabaseResponse デシリアライズ
#[test]
fn test_database_response_deserialize() {
    let json = r#"{"data_sources": [{"id": "ds-123"}]}"#;
    let resp: DatabaseResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.data_sources.len(), 1);
    assert_eq!(resp.data_sources[0].id, "ds-123");
}

// T-022: FR-017 — data_sources 空
#[test]
fn test_database_response_empty_data_sources() {
    let json = r#"{"data_sources": []}"#;
    let resp: DatabaseResponse = serde_json::from_str(json).unwrap();
    assert!(
        resp.data_sources.is_empty(),
        "expected empty, got: {:?}",
        resp.data_sources
    );
}

// T-003 prep: DataSourceQueryResponse デシリアライズ
#[test]
fn test_query_response_deserialize() {
    let json = r#"{
        "results": [{"id": "page-1", "properties": {"Name": {"type": "title", "title": [{"plain_text": "Task"}]}}}],
        "has_more": false,
        "next_cursor": null
    }"#;
    let resp: DataSourceQueryResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.results.len(), 1);
    assert_eq!(resp.results[0].id, "page-1");
    assert_eq!(resp.results[0].properties.property_text("Name"), "Task");
}

// T-024: FR-012, FR-013 — TSV ヘッダー + データ行
#[test]
fn test_tsv_output_format() {
    let json = r#"{
        "results": [{
            "id": "p1",
            "properties": {
                "Name": {"type": "title", "title": [{"plain_text": "Task A"}]},
                "Status": {"type": "select", "select": {"name": "Done"}},
                "Date": {"type": "date", "date": {"start": "2026-03-16", "end": null}}
            }
        }],
        "has_more": false,
        "next_cursor": null
    }"#;
    let resp: DataSourceQueryResponse = serde_json::from_str(json).unwrap();
    let columns = resp.results[0].properties.sorted_names();
    assert_eq!(columns, vec!["Name", "Date", "Status"]);

    let header = format!("id\t{}", columns.join("\t"));
    assert_eq!(header, "id\tName\tDate\tStatus");

    let values: Vec<String> = columns
        .iter()
        .map(|col| resp.results[0].properties.property_text(col))
        .collect();
    let row = format!("{}\t{}", resp.results[0].id, values.join("\t"));
    assert_eq!(row, "p1\tTask A\t2026-03-16\tDone");
}

// T-023: FR-015 — 値内タブをスペースに変換
#[test]
fn test_property_text_tab_in_value() {
    let json = r#"{"Name": {"type": "title", "title": [{"plain_text": "hello\tworld"}]}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Name"), "hello world");
}

// T-021: FR-014 — sorted_names: title 先頭
#[test]
fn test_sorted_names_title_first() {
    let json = r#"{
        "Status": {"type": "select", "select": null},
        "Name": {"type": "title", "title": []},
        "Date": {"type": "date", "date": null}
    }"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.sorted_names(), vec!["Name", "Date", "Status"]);
}

// T-027: FR-008 — date null
#[test]
fn test_property_text_date_null() {
    let json = r#"{"Due": {"type": "date", "date": null}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Due"), "");
}

// T-028: FR-006 — status null
#[test]
fn test_property_text_status_null() {
    let json = r#"{"State": {"type": "status", "status": null}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("State"), "");
}

// T-029: FR-007 — multi_select 空配列
#[test]
fn test_property_text_multi_select_empty() {
    let json = r#"{"Tags": {"type": "multi_select", "multi_select": []}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Tags"), "");
}

// T-030: FR-015 — 値内改行をスペースに変換
#[test]
fn test_property_text_newline_in_value() {
    let json = r#"{"Name": {"type": "title", "title": [{"plain_text": "line1\nline2"}]}}"#;
    let props: PageProperties = serde_json::from_str(json).unwrap();
    assert_eq!(props.property_text("Name"), "line1 line2");
}
