# Spec: notch query — Notion データベースクエリ

## Functional Requirements

| ID     | Requirement                                               | Implements |
| ------ | --------------------------------------------------------- | ---------- |
| FR-001 | `notch query <db-id-or-url>` コマンドで DB をクエリできる | AC-1, AC-2 |
| FR-002 | `GET /v1/databases/{db_id}` で data_source_id を取得する  | AC-1       |
| FR-003 | `POST /v1/data_sources/{ds_id}/query` で行を取得する      | AC-2       |
| FR-004 | title, rich_text → plain_text 結合                        | AC-3       |
| FR-005 | number → 数値文字列（null → 空）                          | AC-3       |
| FR-006 | select, status → name（null → 空）                        | AC-3       |
| FR-007 | multi_select → カンマ区切り name                          | AC-3       |
| FR-008 | date → start（range: start → end）                        | AC-3       |
| FR-009 | checkbox → "true" / "false"                               | AC-3       |
| FR-010 | url, email, phone_number → そのまま（null → 空）          | AC-3       |
| FR-011 | 未対応プロパティ型 → 空文字列                             | AC-3       |
| FR-012 | 出力 1 行目: ヘッダー `id\t<col1>\t<col2>...`             | AC-4       |
| FR-013 | 出力 2 行目以降: `<page_id>\t<val1>\t<val2>...`           | AC-4       |
| FR-014 | カラム順: title 列先頭、残りアルファベット順              | AC-4       |
| FR-015 | 値内タブ・改行をスペースに変換                            | AC-4       |
| FR-016 | 結果 0 件 → stderr に "No rows found"、stdout 出力なし    | AC-2, AC-4 |
| FR-017 | データソースなし → "Database has no data sources" エラー  | AC-5       |
| FR-018 | URL パースは `parse_page_id` を再利用（フォーマット同一） | AC-1       |

## Non-Functional Requirements

| ID      | Requirement                              |
| ------- | ---------------------------------------- |
| NFR-001 | 外部依存追加なし（既存 crate のみ）      |
| NFR-002 | リトライロジック再利用（429, 5xx）       |
| NFR-003 | 未対応プロパティ型は安全に空文字列で通過 |

## Test Scenarios

| ID    | Scenario                                 | Validates      | Input                                                                | Expected                               |
| ----- | ---------------------------------------- | -------------- | -------------------------------------------------------------------- | -------------------------------------- |
| T-001 | retrieve_database が data_source_id 返却 | FR-002         | `GET /v1/databases/{id}` → 200 + data_sources                        | 最初の data_source_id を取得           |
| T-002 | retrieve_database 404                    | FR-002         | `GET /v1/databases/{id}` → 404                                       | NotFoundOrForbidden エラー             |
| T-003 | query_data_source が行を返す             | FR-003         | `POST /v1/data_sources/{id}/query` → 200 + results                   | results にページオブジェクトが含まれる |
| T-004 | query_data_source 空結果                 | FR-003         | `POST /v1/data_sources/{id}/query` → 200 + empty results             | results が空配列                       |
| T-005 | title プロパティ抽出                     | FR-004         | `{"type":"title","title":[{"plain_text":"Hello"}]}`                  | `"Hello"`                              |
| T-006 | rich_text プロパティ抽出                 | FR-004         | `{"type":"rich_text","rich_text":[{"plain_text":"world"}]}`          | `"world"`                              |
| T-007 | number プロパティ                        | FR-005         | `{"type":"number","number":42}`                                      | `"42"`                                 |
| T-008 | number null                              | FR-005         | `{"type":"number","number":null}`                                    | `""`                                   |
| T-009 | select プロパティ                        | FR-006         | `{"type":"select","select":{"name":"Done"}}`                         | `"Done"`                               |
| T-010 | select null                              | FR-006         | `{"type":"select","select":null}`                                    | `""`                                   |
| T-011 | status プロパティ                        | FR-006         | `{"type":"status","status":{"name":"In Progress"}}`                  | `"In Progress"`                        |
| T-012 | multi_select プロパティ                  | FR-007         | `{"type":"multi_select","multi_select":[{"name":"A"},{"name":"B"}]}` | `"A, B"`                               |
| T-013 | date 単一                                | FR-008         | `{"type":"date","date":{"start":"2026-03-16","end":null}}`           | `"2026-03-16"`                         |
| T-014 | date range                               | FR-008         | `{"type":"date","date":{"start":"2026-03-16","end":"2026-03-20"}}`   | `"2026-03-16 → 2026-03-20"`            |
| T-015 | checkbox true                            | FR-009         | `{"type":"checkbox","checkbox":true}`                                | `"true"`                               |
| T-016 | checkbox false                           | FR-009         | `{"type":"checkbox","checkbox":false}`                               | `"false"`                              |
| T-017 | url プロパティ                           | FR-010         | `{"type":"url","url":"https://example.com"}`                         | `"https://example.com"`                |
| T-018 | email プロパティ                         | FR-010         | `{"type":"email","email":"a@b.com"}`                                 | `"a@b.com"`                            |
| T-019 | phone_number プロパティ                  | FR-010         | `{"type":"phone_number","phone_number":"+81-90-1234-5678"}`          | `"+81-90-1234-5678"`                   |
| T-020 | 未対応型は空文字列                       | FR-011         | `{"type":"formula","formula":{"type":"string","string":"x"}}`        | `""`                                   |
| T-021 | sorted_names: title 先頭                 | FR-014         | properties に "Status"(select), "Name"(title), "Date"(date) がある   | `["Name", "Date", "Status"]`           |
| T-022 | NoDataSources エラー                     | FR-017         | data_sources が空配列                                                | "Database has no data sources" エラー  |
| T-023 | 値内タブをスペースに変換                 | FR-015         | title に `"hello\tworld"` を含む                                     | `"hello world"`                        |
| T-024 | TSV ヘッダー + データ行フォーマット      | FR-012, FR-013 | 2 行・3 プロパティ (Name:title, Date:date, Status:select)            | `"id\tName\tDate\tStatus\n<id>\t..."`  |
| T-025 | E2E: parse → retrieve DB → query → TSV   | FR-001         | wiremock で全 API をモック、TSV stdout を検証                        | ヘッダー + データ行が正しい            |
| T-026 | 0 件時 stdout 出力なし                   | FR-016         | query_data_source が空 results を返す                                | stdout が空、stderr に "No rows found" |
| T-027 | date null                                | FR-008         | `{"type":"date","date":null}`                                        | `""`                                   |
| T-028 | status null                              | FR-006         | `{"type":"status","status":null}`                                    | `""`                                   |
| T-029 | multi_select 空配列                      | FR-007         | `{"type":"multi_select","multi_select":[]}`                          | `""`                                   |
| T-030 | 値内改行をスペースに変換                 | FR-015         | title に `"line1\nline2"` を含む                                     | `"line1 line2"`                        |

## Traceability Matrix

| AC   | FR                     | Test                       | NFR     |
| ---- | ---------------------- | -------------------------- | ------- |
| AC-1 | FR-001, FR-002, FR-018 | T-001, T-002, T-025        | —       |
| AC-2 | FR-001, FR-003, FR-016 | T-003, T-004, T-025, T-026 | NFR-002 |
| AC-3 | FR-004〜FR-011         | T-005〜T-020               | NFR-003 |
| AC-4 | FR-012〜FR-016         | T-021, T-023〜T-026        | —       |
| AC-5 | FR-017                 | T-022                      | —       |

## Component API

> [→] `data_sources` フィールド構造は Notion upgrade guide 2025-09-03 に基づく仮定。実装前に実 API で検証する。

```rust
// src/types.rs (追加)
pub struct DatabaseResponse {
    pub data_sources: Vec<DataSourceInfo>,
}

pub struct DataSourceInfo {
    pub id: String,
}

pub struct DataSourceQueryResponse {
    pub results: Vec<QueryResult>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

pub struct QueryResult {
    pub id: String,
    pub properties: PageProperties,
}

// src/types.rs (PageProperties 拡張)
impl PageProperties {
    /// 指定プロパティの plain text 値を返す
    pub fn property_text(&self, key: &str) -> String;

    /// プロパティ名をソートして返す（title 先頭 → 残りアルファベット順）
    pub fn sorted_names(&self) -> Vec<String>;
}

// src/client.rs (追加)
impl Client {
    pub async fn retrieve_database(&self, db_id: &str) -> Result<DatabaseResponse, NotchError>;
    pub async fn query_data_source(&self, ds_id: &str) -> Result<DataSourceQueryResponse, NotchError>;
}

// src/client.rs (エラー追加)
pub enum NotchError {
    // ...existing...
    #[error("Database has no data sources")]
    NoDataSources,
}

// NoDataSources は main.rs run() で data_sources.is_empty() をチェックして返す
```
