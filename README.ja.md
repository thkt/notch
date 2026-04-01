[English](README.md) | **日本語**

# notch

NotionページをMarkdownで取得 — 人間にもAIエージェントにも。

## 課題

Notionページの内容をMarkdownとして使いたい。

**notch なし:**

```
# 公式 CLI がない。選択肢は:
# 1. ブラウザからコピペ（書式が崩れる）
# 2. API を直接叩く（ページネーション、認証、JSON パース）
# 3. Markdown エクスポート（手動、自動化不可）
```

**notch あり:**

```sh
notch fetch https://www.notion.so/My-Page-abc123def456...

# My Page

きれいな Markdown で出力。パイプにそのまま渡せる。
```

コマンド1つ。URLパース、API認証、タイトル抽出、Markdown出力をすべて処理。

## notch を使うべきとき（使わないべきとき）

**notch が向いているケース:**

- ターミナルやスクリプトでNotionページをMarkdownとして取得したい
- Notionの内容を他のツールにパイプしたい（`notch fetch ... | grep pattern`）
- コマンドラインからNotionワークスペースを検索したい
- Notionデータベースの行をTSVで取得したい

**他のツールが向いているケース:**

- Notionページの編集・作成 — notchは読み取り専用
- データベースのフィルタリングやソート — notchは最初のページの結果のみ返す
- リアルタイム同期 — notchはワンショット取得

## セットアップ

### インストール

```sh
brew install thkt/tap/notch
```

ソースからビルド:

```sh
cargo install --path .
```

ビルド済みバイナリは [Releases](https://github.com/thkt/notch/releases) から — macOS (Apple Silicon / Intel), Linux (x86_64 / ARM64)。

### 環境変数

```sh
export NOTION_TOKEN="ntn_..."  # 必須: https://www.notion.so/profile/integrations
```

内部インテグレーションを作成し、対象ページをインテグレーションに共有してください。

#### 設定場所

**シェル（ターミナルから使う場合）:**

```sh
# ~/.zshenv に追記
export NOTION_TOKEN="ntn_..."
```

**Claude Code（ツール / MCP 連携で使う場合）:**

`~/.claude/settings.json` に追加:

```json
{
  "env": {
    "NOTION_TOKEN": "ntn_..."
  }
}
```

### Claude Code 連携

プロジェクトの `CLAUDE.md` に追加:

```markdown
## Tools

- `notch fetch <page-id-or-url>` — Notion ページを Markdown で取得
- `notch search "query"` — Notion ページをタイトルで検索
- `notch query <database-id-or-url>` — Notion データベースの行を TSV で取得
```

## コマンド

### `notch fetch` — Notion ページを Markdown で取得

Notion APIのネイティブMarkdownエンドポイント経由でページを取得。メタデータからタイトルを抽出し、`# H1` 見出しとして先頭に付加。

```sh
notch fetch https://www.notion.so/My-Page-abc123def456...
notch fetch abc123def456...                    # hex32 ID
notch fetch 12345678-1234-1234-1234-1234567890ab  # UUID
echo "abc123def456..." | notch fetch           # stdin
```

Notion URL（`www` 有無、`notion.site` サブドメイン、`?p=` クエリパラメータ対応）、生のUUID、32文字のhex IDを受け付けます。

警告はstderrに出力:

- ページが切り詰められた場合（Notion APIの制限）
- Markdownに変換できなかったブロック
- 出力が100KBを超えた場合（UTF-8境界で切り詰め）

### `notch search` — Notion ページ検索

ワークスペースをタイトルで検索。タブ区切りで結果を返す: ページID、タイトル、最終編集日時。

```sh
notch search "議事録"

  abc123...  週次定例  2026-03-13T10:00:00.000Z
  def456...  1on1 メモ  2026-03-12T15:30:00.000Z
```

スクリプト向けのパイプフレンドリーな出力:

```sh
notch search "RFC" | head -5           # 最初の5件
notch search "draft" | cut -f1         # ページ ID のみ
notch search "仕様" | while read -r id title _; do
  notch fetch "$id" > "$title.md"
done
```

### `notch query` — Notion データベースクエリ

Notion Data Source API経由でデータベースをクエリし、行をTSV（タブ区切り）で出力。

```sh
notch query https://www.notion.so/My-Database-abc123def456...
notch query abc123def456...                    # hex32 ID
echo "abc123def456..." | notch query           # stdin
```

出力形式: 1行目がヘッダー、2行目以降がデータ行。

```
id	名前	ステータス	日付
abc123...	タスクA	完了	2026-03-15
def456...	タスクB	進行中	2026-03-14
```

カラム順: タイトル列が先頭、残りはアルファベット順。1列目は常にページID。

スクリプト向けのパイプフレンドリーな出力:

```sh
notch query <db-url> | tail -n +2 | cut -f1    # ページ ID のみ（ヘッダースキップ）
notch query <db-url> | cut -f2                  # タイトル列のみ
notch query <db-url> | tail -n +2 | while IFS=$'\t' read -r id title _; do
  notch fetch "$id" > "$title.md"
done
```

対応プロパティ型: title, rich_text, number, select, multi_select, date, checkbox, url, email, phone_number, status。未対応型は空文字として出力。

## 仕組み

1. **URL パース** — Notion URL、hex32文字列、UUIDからページIDを抽出。`notion.so` と `notion.site`（サブドメイン対応）に対してドメインを検証。
2. **並列フェッチ** — `fetch_markdown` と `fetch_metadata` を `tokio::try_join!` で同時実行。
3. **リトライ** — 429（レート制限）は `Retry-After` ヘッダーに従ってリトライ。5xxエラーは指数バックオフ（100ms, 200ms, 400ms）で最大3回リトライ。
4. **タイトル抽出** — ページメタデータからタイトルプロパティを動的に検出（プロパティ名に依存しない）。
5. **サニタイズ** — Notion独自のカスタムタグ（`<empty-block/>`、`{color="..."}`、`<span>`、`<mention-*>`、`<checkbox>`、`<callout>`、`<details>` 等）を除去し、HTMLテーブルをMarkdownパイプテーブルに変換。LLMコンテキスト入力に最適化。
6. **出力** — `# Title` 見出しを先頭に付加、100KBでUTF-8境界安全に切り詰め。

## アーキテクチャ

```
src/
├── main.rs       CLI エントリポイント（clap）、SIGPIPE 処理
├── client.rs     Notion API クライアント、URL パース、リトライロジック
├── types.rs      API レスポンス型、プロパティ抽出
├── sanitize.rs   Notion カスタムタグ除去、HTML→Markdown 変換
├── markdown.rs   出力フォーマット、切り詰め
└── lib.rs        モジュール再エクスポート
```

シングルバイナリ、ランタイム依存ゼロ。

## 制限事項

| 制限                  | 詳細                                                                                              |
| --------------------- | ------------------------------------------------------------------------------------------------- |
| Notion トークンが必要 | notion.so/profile/integrations でインテグレーションを作成。ページをインテグレーションに共有が必要 |
| 読み取り専用          | ページの作成・編集は不可                                                                          |
| ページネーションなし  | 検索・クエリは最初のページの結果のみ返す                                                          |
| Notion API レート制限 | インテグレーションあたり 3 リクエスト/秒。429 は自動リトライ                                      |
| 出力サイズ上限        | 最大 100KB、UTF-8 境界で切り詰め                                                                  |

## ライセンス

MIT
