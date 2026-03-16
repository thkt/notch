# SOW: notch query — Notion データベースクエリ

## Status

draft

## Overview

| Field     | Value                                                                        |
| --------- | ---------------------------------------------------------------------------- |
| Purpose   | Notion データベースの行を TSV で出力する `notch query` コマンドを追加する    |
| Target    | `src/main.rs`, `src/client.rs`, `src/types.rs`, `tests/api_test.rs`          |
| Approach  | 新 Data Source API（2025-09-03+）を使用。DB ID → data_source_id 解決後クエリ |
| Reference | Notion API v2026-03-11, upgrade guide 2025-09-03                             |

## Background

notchは現在 `fetch`（ページ→Markdown）と `search`（タイトル検索）をサポートしている。Notionデータベース（テーブル）のクエリは未対応。API version 2025-09-03でデータベースとデータソースが分離され、クエリはData Source API（`POST /v1/data_sources/{id}/query`）に移行した。notchはversion 2026-03-11を使用しているため、Data Source APIを使う。

## Scope

### In Scope

| Target      | Change                                            | Files |
| ----------- | ------------------------------------------------- | ----- |
| main.rs     | `Query` サブコマンド追加                          | 1     |
| client.rs   | `retrieve_database()`, `query_data_source()` 追加 | 1     |
| types.rs    | レスポンス型 + プロパティ値抽出ロジック           | 1     |
| api_test.rs | 新エンドポイントの wiremock テスト                | 1     |

### Out of Scope

- フィルター・ソートオプション（将来対応候補）
- ページネーション（1ページ目のみ、`search` と一致）
- 複数データソース対応（最初のデータソースのみ）
- データベーススキーマ表示（別コマンド候補）
- `parse_page_id` のリネーム（URLフォーマット同一で再利用可）

## Acceptance Criteria

### AC-1: データベース ID の解決

- [ ] Notion URL、hex32、UUIDからデータベースIDを抽出できる（`parse_page_id` 再利用）
- [ ] `GET /v1/databases/{db_id}` でデータソースIDを取得できる
- [ ] データソースが存在しない場合、エラーメッセージを表示する

### AC-2: データソースクエリの実行

- [ ] `POST /v1/data_sources/{ds_id}/query` で行を取得できる
- [ ] リトライロジック（429, 5xx）が適用される（`send_with_retry` 再利用）
- [ ] 結果が0件の場合、stderrに "No rows found" を表示する

### AC-3: プロパティ値の plain text 変換

- [ ] title, rich_text → plain_textを結合
- [ ] number → 数値文字列
- [ ] select, status → name
- [ ] multi_select → カンマ区切りname
- [ ] date → start（rangeの場合start → end）
- [ ] checkbox → true/false
- [ ] url, email, phone_number → そのまま
- [ ] 未対応型 → 空文字列

### AC-4: TSV 出力フォーマット

- [ ] 1行目: ヘッダー（`id\t<col1>\t<col2>\t...`）
- [ ] 2行目以降: データ行（`<page_id>\t<val1>\t<val2>\t...`）
- [ ] カラム順: title列を先頭、残りはアルファベット順
- [ ] 値内のタブ・改行はスペースに変換
- [ ] 結果が0件の場合、ヘッダーも出力しない

### AC-5: エラーハンドリング

- [ ] データベース未共有 → NotFoundOrForbiddenエラー
- [ ] データソースなし → 専用エラーメッセージ
- [ ] APIエラー → 既存のエラーハンドリングで処理

## Implementation Plan

### Phase 1: types.rs — レスポンス型 + プロパティ値抽出

1. `DatabaseResponse` 構造体追加（data_sourcesリスト）
2. `DataSourceInfo` 構造体追加（id, name）
3. `DataSourceQueryResponse` 構造体追加（results, has_more, next_cursor）
4. `QueryResult` 構造体追加（id, properties）
5. `PageProperties` に `property_text()` メソッド追加（型別のplain text変換）
6. `PageProperties` に `sorted_names()` メソッド追加（title先頭 → 残りアルファベット順）
7. Validates: AC-3

### Phase 2: client.rs — API クライアント

1. `retrieve_database()` メソッド追加: `GET /v1/databases/{db_id}`
2. `query_data_source()` メソッド追加: `POST /v1/data_sources/{ds_id}/query`
3. 両メソッドとも `send_with_retry` + `handle_response` を再利用
4. `NotchError` に `NoDataSources` バリアント追加
5. `NoDataSources` は `main.rs` の `run()` で `data_sources.is_empty()` をチェックして返す
6. Validates: AC-1, AC-2, AC-5

### Phase 3: main.rs — CLI コマンド

1. `Commands` enumに `Query` バリアント追加
2. `run()` に `Query` マッチアーム追加
3. フロー: parse ID → retrieve DB → query DS → format TSV output
4. 値内のタブ・改行をスペースに変換して出力
5. Validates: AC-4

### Phase 4: テスト

1. `types.rs` 単体テスト: プロパティ値抽出の各型
2. `api_test.rs` 統合テスト: retrieve_database, query_data_sourceのwiremockテスト
3. Validates: 全AC

## Test Plan

Spec Test Scenarios (T-001〜T-026) が正式。以下はカテゴリ別サマリ。

| Category              | Tests        | AC         | Target      |
| --------------------- | ------------ | ---------- | ----------- |
| DB 解決 + クエリ      | T-001〜T-004 | AC-1, AC-2 | api_test.rs |
| プロパティ値抽出      | T-005〜T-020 | AC-3       | types.rs    |
| カラム順              | T-021        | AC-4       | types.rs    |
| エラーハンドリング    | T-022        | AC-5       | types.rs    |
| TSV フォーマット      | T-023〜T-024 | AC-4       | types.rs    |
| E2E（クエリ全フロー） | T-025        | ALL        | api_test.rs |
| 0 件出力なし          | T-026        | AC-4       | api_test.rs |

## Risks

| Risk                              | Impact | Mitigation                                 |
| --------------------------------- | ------ | ------------------------------------------ |
| 新 API レスポンス形式の差異       | MED    | wiremock テストで形式を固定。実 API で検証 |
| data_sources フィールド名の不一致 | MED    | Notion 公式ドキュメント + 実 API で確認    |
| プロパティ型の追加・変更          | LOW    | 未対応型は空文字列で安全に通過             |
| 大量行のデータベース（100 件超）  | LOW    | 1 ページ目のみ。制限として README に文書化 |
