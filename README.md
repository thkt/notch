**English** | [日本語](README.ja.md)

# notch

Fetch Notion pages as Markdown — for humans and AI agents alike.

## The problem

You need the content of a Notion page as Markdown for your workflow.

**Without notch:**

```
# No official CLI. You either:
# 1. Copy-paste from the browser (loses formatting)
# 2. Use the API manually (pagination, auth, JSON parsing)
# 3. Export as Markdown (manual, no automation)
```

**With notch:**

```sh
notch fetch https://www.notion.so/My-Page-abc123def456...

# My Page

Content as clean Markdown, ready to pipe.
```

One command. Handles URL parsing, API authentication, title extraction, and Markdown output.

## When to use notch (and when not to)

**Use notch when:**

- You need Notion page content as Markdown in a terminal or script
- You want to pipe Notion content into other tools (`notch fetch ... | grep pattern`)
- You need to search across your Notion workspace from the command line
- You need to query a Notion database and get rows as TSV

**Use other tools when:**

- You need to edit or create Notion pages — notch is read-only
- You need database filtering or sorting — notch returns the first page of results
- You need real-time sync — notch is a one-shot fetch

## Setup

### Install

```sh
brew install thkt/tap/notch
```

Or build from source:

```sh
cargo install --path .
```

Pre-built binaries in [Releases](https://github.com/thkt/notch/releases) — macOS (Apple Silicon / Intel), Linux (x86_64 / ARM64).

### Environment

```sh
export NOTION_TOKEN="ntn_..."  # Required: https://www.notion.so/profile/integrations
```

Create an internal integration, then share the target pages with it.

#### Where to set

**Shell (terminal usage):**

```sh
# Add to ~/.zshenv
export NOTION_TOKEN="ntn_..."
```

**Claude Code (tool / MCP usage):**

Add to `~/.claude/settings.json`:

```json
{
  "env": {
    "NOTION_TOKEN": "ntn_..."
  }
}
```

### Claude Code integration

Add to your project's `CLAUDE.md`:

```markdown
## Tools

- `notch fetch <page-id-or-url>` — Notion page to Markdown
- `notch search "query"` — search Notion pages by title
- `notch query <database-id-or-url>` — query Notion database rows as TSV
```

## Commands

### `notch fetch` — Notion page to Markdown

Fetches a page via the Notion API's native Markdown endpoint. Extracts the title from metadata and prepends it as an `# H1` heading.

```sh
notch fetch https://www.notion.so/My-Page-abc123def456...
notch fetch abc123def456...                    # hex32 ID
notch fetch 12345678-1234-1234-1234-1234567890ab  # UUID
echo "abc123def456..." | notch fetch           # stdin
notch fetch -                                  # explicit stdin
```

Accepts Notion URLs (with or without `www`, `notion.site` subdomains, `?p=` query params), raw UUIDs, and 32-character hex IDs.

Warnings are printed to stderr:

- Truncated pages (Notion API limit)
- Blocks that couldn't be converted to Markdown
- Output exceeding 100KB (truncated at UTF-8 boundary)

### `notch search` — Search Notion pages

Searches your workspace by title. Returns tab-separated results: page ID, title, last edited time.

```sh
notch search "meeting notes"

  abc123...  Weekly Sync  2026-03-13T10:00:00.000Z
  def456...  1:1 Notes    2026-03-12T15:30:00.000Z
```

Pipe-friendly output for scripting:

```sh
notch search "RFC" | head -5           # first 5 results
notch search "draft" | cut -f1         # page IDs only
notch search "spec" | while read -r id title _; do
  notch fetch "$id" > "$title.md"
done
```

### `notch query` — Query a Notion database

Queries a database via the Notion Data Source API and outputs rows as TSV (tab-separated values).

```sh
notch query https://www.notion.so/My-Database-abc123def456...
notch query abc123def456...                    # hex32 ID
echo "abc123def456..." | notch query           # stdin
notch query -                                  # explicit stdin
```

Output format: first line is a header row, subsequent lines are data rows.

```
id	Name	Status	Date
abc123...	Task A	Done	2026-03-15
def456...	Task B	In Progress	2026-03-14
```

Column order: title column first, remaining columns alphabetically. First column is always the page ID.

Pipe-friendly output for scripting:

```sh
notch query <db-url> | tail -n +2 | cut -f1    # page IDs only (skip header)
notch query <db-url> | cut -f2                  # title column only
notch query <db-url> | tail -n +2 | while IFS=$'\t' read -r id title _; do
  notch fetch "$id" > "$title.md"
done
```

Supported property types: title, rich_text, number, select, multi_select, date, checkbox, url, email, phone_number, status. Unsupported types output as empty.

## How it works

1. **URL parsing** — Extracts page ID from Notion URLs, hex32 strings, or UUIDs. Validates domain against `notion.so` and `notion.site` (with subdomain support).
2. **Parallel fetch** — `fetch_markdown` and `fetch_metadata` run concurrently via `tokio::try_join!`.
3. **Retry** — 429 (rate-limited) responses retry using the `Retry-After` header. 5xx errors retry with exponential backoff (100ms, 200ms, 400ms). Up to 3 retries.
4. **Title extraction** — Dynamically finds the title property from page metadata (works regardless of property name).
5. **Sanitize** — Removes Notion-specific custom tags (`<empty-block/>`, `{color="..."}`, `<span>`, `<mention-*>`, `<checkbox>`, `<callout>`, `<details>`, etc.), converts HTML tables to Markdown pipe tables, and optimizes output for LLM consumption.
6. **Output** — Prepends `# Title` heading, truncates at 100KB with UTF-8 boundary safety.

## Architecture

```
src/
├── main.rs       CLI entry point (clap), SIGPIPE handling
├── client.rs     Notion API client, URL parsing, retry logic
├── types.rs      API response types, property extraction
├── sanitize.rs   Notion custom tag removal, HTML→Markdown conversion
├── markdown.rs   Output formatting, truncation
└── lib.rs        Module re-exports
```

Single binary, zero runtime dependencies.

## Limitations

| Limitation             | Details                                                                                            |
| ---------------------- | -------------------------------------------------------------------------------------------------- |
| Notion token required  | Create an integration at notion.so/profile/integrations. Pages must be shared with the integration |
| Read-only              | No page creation or editing                                                                        |
| No pagination          | Search and query return the first page of results only                                             |
| Notion API rate limits | 3 requests/second per integration. notch retries on 429 automatically                              |
| Output size cap        | 100KB max output, truncated at UTF-8 boundary                                                      |

## License

MIT
