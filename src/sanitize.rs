use std::borrow::Cow;

/// Sanitize Notion API markdown output by removing/converting non-standard tags and attributes.
pub fn sanitize(input: &str) -> String {
    let segments = split_code_segments(input);
    let mut result = String::with_capacity(input.len());

    for seg in segments {
        match seg {
            Segment::Code(code) => result.push_str(code),
            Segment::Text(text) => {
                let s = sanitize_text(text);
                result.push_str(&s);
            }
        }
    }

    normalize_blank_lines(&result)
}

enum Segment<'a> {
    Code(&'a str),
    Text(&'a str),
}

/// Split input into code (fenced blocks + inline code) and text segments.
/// Backtick is ASCII (0x60), so byte offsets from find('`') are always valid char boundaries.
fn split_code_segments(input: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        if let Some(pos) = remaining.find('`') {
            if pos > 0 {
                segments.push(Segment::Text(&remaining[..pos]));
            }
            remaining = &remaining[pos..];

            if remaining.starts_with("```") {
                match remaining[3..].find("```") {
                    Some(end) => {
                        segments.push(Segment::Code(&remaining[..end + 6]));
                        remaining = &remaining[end + 6..];
                    }
                    None => {
                        segments.push(Segment::Code(remaining));
                        remaining = "";
                    }
                }
            } else {
                match remaining[1..].find('`') {
                    Some(end) => {
                        segments.push(Segment::Code(&remaining[..end + 2]));
                        remaining = &remaining[end + 2..];
                    }
                    None => {
                        segments.push(Segment::Text(&remaining[..1]));
                        remaining = &remaining[1..];
                    }
                }
            }
        } else {
            segments.push(Segment::Text(remaining));
            remaining = "";
        }
    }

    segments
}

/// Sanitize non-code text segments.
fn sanitize_text(input: &str) -> String {
    let mut s = input.to_string();

    // Table conversion must run first: it produces markdown that later
    // steps (span stripping, br removal) would otherwise miss inside cells.
    convert_tables_mut(&mut s);
    convert_mentions_mut(&mut s);
    strip_span_tags_mut(&mut s);

    if s.contains("<empty-block/>") {
        s = s.replace("<empty-block/>", "\n\n");
    }
    if s.contains("<divider/>") {
        s = s.replace("<divider/>", "---");
    }
    if s.contains("<br") {
        s = s.replace("<br/>", "\n");
        s = s.replace("<br>", "\n");
    }

    strip_curly_attributes_mut(&mut s);
    strip_image_signatures_mut(&mut s);

    s
}

/// Replace all occurrences of a custom tag with a computed replacement.
fn replace_tag(s: &mut String, tag_name: &str, replacement: impl Fn(&str) -> String) {
    let open = format!("<{tag_name}");
    let mut search_from = 0;
    while let Some(pos) = s[search_from..].find(&open) {
        let start = search_from + pos;
        let Some(end) = find_tag_end(s, start, tag_name) else {
            break;
        };
        let tag = s[start..end].to_string();
        let repl = replacement(&tag);
        let repl_len = repl.len();
        s.replace_range(start..end, &repl);
        search_from = start + repl_len;
    }
}

/// Convert <mention-page>, <mention-user>, <file> tags in-place.
fn convert_mentions_mut(s: &mut String) {
    replace_tag(s, "mention-page", |tag| {
        let url = extract_attr(tag, "url").unwrap_or_default();
        format!("[Notion page]({url})")
    });
    replace_tag(s, "mention-user", |_| "@user".to_string());
    replace_tag(s, "file", |_| "[attachment]".to_string());
}

/// Find the end position (exclusive) of a tag, handling self-closing and paired tags.
fn find_tag_end(s: &str, start: usize, tag_name: &str) -> Option<usize> {
    let after_tag = &s[start..];

    if let Some(close) = after_tag.find("/>") {
        let gt = after_tag.find('>');
        if gt.is_none_or(|g| close <= g) {
            return Some(start + close + 2);
        }
    }

    if let Some(gt) = after_tag.find('>') {
        let closing = format!("</{tag_name}>");
        if let Some(close_start) = s[start + gt + 1..].find(&closing) {
            return Some(start + gt + 1 + close_start + closing.len());
        }
        return Some(start + gt + 1);
    }

    None
}

/// Extract an attribute value from a tag string: attr="value"
fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=\"");
    let start = tag.find(&pattern)?;
    let value_start = start + pattern.len();
    let value_end = tag[value_start..].find('"')?;
    Some(tag[value_start..value_start + value_end].to_string())
}

/// Strip <span color="...">text</span> → text, in-place.
fn strip_span_tags_mut(s: &mut String) {
    while let Some(start) = s.find("<span ") {
        let Some(gt) = s[start..].find('>') else {
            break;
        };
        let tag_end = start + gt + 1;

        if let Some(close) = s[tag_end..].find("</span>") {
            let content = s[tag_end..tag_end + close].to_string();
            s.replace_range(start..tag_end + close + 7, &content);
        } else {
            s.replace_range(start..tag_end, "");
        }
    }
}

/// Strip {color="..."}, {toggle="true"} etc. from end of lines, in-place.
fn strip_curly_attributes_mut(s: &mut String) {
    if !s.contains('{') {
        return;
    }
    let mut result = String::with_capacity(s.len());

    for line in s.split('\n') {
        let cleaned = strip_curly_from_line(line);
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&cleaned);
    }

    *s = result;
}

fn strip_curly_from_line(line: &str) -> Cow<'_, str> {
    let trimmed = line.trim_end();
    if let Some(brace_start) = trimmed.rfind('{') {
        let after_brace = &trimmed[brace_start..];
        if after_brace.ends_with('}') && after_brace.contains('=') {
            return Cow::Owned(trimmed[..brace_start].trim_end().to_string());
        }
    }
    Cow::Borrowed(line)
}

/// Strip S3 presigned URL query params from image markdown, in-place.
fn strip_image_signatures_mut(s: &mut String) {
    let marker = "?X-Amz-";
    let mut search_from = 0;
    while let Some(img_start) = s[search_from..].find("![") {
        let abs_start = search_from + img_start;
        let Some(paren_open) = s[abs_start..].find('(') else {
            search_from = abs_start + 2;
            continue;
        };
        let paren_pos = abs_start + paren_open;
        let Some(paren_close) = s[paren_pos..].find(')') else {
            search_from = paren_pos + 1;
            continue;
        };
        let url_start = paren_pos + 1;
        let url_end = paren_pos + paren_close;
        let url = &s[url_start..url_end];

        if let Some(q) = url.find(marker) {
            let clean_end = url_start + q;
            s.replace_range(clean_end..url_end, "");
            search_from = clean_end + 1;
        } else {
            search_from = url_end + 1;
        }
    }
}

/// Convert HTML <table> blocks to Markdown pipe tables, in-place.
fn convert_tables_mut(s: &mut String) {
    if !s.contains("<table>") {
        return;
    }

    let mut result = String::with_capacity(s.len());
    let mut remaining = s.as_str();

    while let Some(table_start) = remaining.find("<table>") {
        result.push_str(&remaining[..table_start]);

        let after = &remaining[table_start..];
        if let Some(table_end_offset) = after.find("</table>") {
            let table_content = &after[7..table_end_offset];
            let rows = parse_table_rows(table_content);
            if !rows.is_empty() {
                result.push_str(&render_pipe_table(&rows));
            }
            remaining = &after[table_end_offset + 8..];
        } else {
            result.push_str(after);
            remaining = "";
        }
    }

    result.push_str(remaining);
    *s = result;
}

/// Parse HTML table rows into a 2D string grid.
fn parse_table_rows(html: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let cleaned = remove_colgroup(html);
    let mut remaining: &str = &cleaned;

    while let Some(tr_start) = remaining.find("<tr>") {
        let after_tr = &remaining[tr_start + 4..];
        if let Some(tr_end) = after_tr.find("</tr>") {
            let cells = extract_cells(&after_tr[..tr_end]);
            if !cells.is_empty() {
                rows.push(cells);
            }
            remaining = &after_tr[tr_end + 5..];
        } else {
            break;
        }
    }

    rows
}

/// Render a 2D string grid as a Markdown pipe table.
fn render_pipe_table(rows: &[Vec<String>]) -> String {
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut lines = Vec::with_capacity(rows.len() + 1);

    for (i, row) in rows.iter().enumerate() {
        let mut cells = row.clone();
        while cells.len() < col_count {
            cells.push(String::new());
        }
        lines.push(format!("| {} |", cells.join(" | ")));

        if i == 0 {
            lines.push(format!("| {} |", vec!["---"; col_count].join(" | ")));
        }
    }

    lines.join("\n")
}

/// Remove <colgroup>...</colgroup> from HTML.
fn remove_colgroup(html: &str) -> Cow<'_, str> {
    if !html.contains("<colgroup>") {
        return Cow::Borrowed(html);
    }
    let mut s = html.to_string();
    while let Some(start) = s.find("<colgroup>") {
        if let Some(end) = s[start..].find("</colgroup>") {
            s.replace_range(start..start + end + 11, "");
        } else {
            break;
        }
    }
    Cow::Owned(s)
}

/// Extract cell contents from a <tr> row.
fn extract_cells(row_html: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut remaining = row_html;

    while let Some(td_start) = remaining.find("<td") {
        let after_td = &remaining[td_start..];
        let Some(gt) = after_td.find('>') else { break };
        let content_start = td_start + gt + 1;

        if let Some(td_end) = remaining[content_start..].find("</td>") {
            let raw = &remaining[content_start..content_start + td_end];
            cells.push(clean_cell_content(raw));
            remaining = &remaining[content_start + td_end + 5..];
        } else {
            break;
        }
    }

    cells
}

/// Clean cell content: strip inline tags, escape pipes, remove <br>.
fn clean_cell_content(content: &str) -> String {
    let trimmed = content.trim();
    // Fast path: no special markers
    if !trimmed.contains('<') && !trimmed.contains('|') {
        return trimmed.to_string();
    }

    let mut s = trimmed.to_string();
    if s.contains("<span ") {
        strip_span_tags_mut(&mut s);
    }
    if s.contains("<br") {
        s = s.replace("<br/>", " ");
        s = s.replace("<br>", " ");
    }
    if s.contains('|') {
        s = s.replace('|', r"\|");
    }
    s.trim().to_string()
}

/// Normalize consecutive blank lines to max 2 (one blank line between content).
fn normalize_blank_lines(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut consecutive_newlines = 0;

    for ch in input.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 3 {
                result.push(ch);
            }
        } else {
            consecutive_newlines = 0;
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // T-001: FR-001 — empty-block を空行に変換
    #[test]
    fn test_empty_block_to_blank_line() {
        assert_eq!(sanitize("text<empty-block/>more"), "text\n\nmore");
    }

    // T-002: FR-002 — color 属性を除去
    #[test]
    fn test_color_attribute_removed() {
        assert_eq!(sanitize("## Title {color=\"gray_bg\"}"), "## Title");
    }

    // T-003: FR-002 — toggle 属性を除去
    #[test]
    fn test_toggle_attribute_removed() {
        assert_eq!(sanitize("## Title {toggle=\"true\"}"), "## Title");
    }

    // T-004: FR-003 — span color をテキストに変換
    #[test]
    fn test_span_color_to_text() {
        assert_eq!(
            sanitize("<span color=\"red\">important</span>"),
            "important"
        );
    }

    // T-005: FR-004 — br を改行に変換
    #[test]
    fn test_br_to_newline() {
        assert_eq!(sanitize("line1<br>line2"), "line1\nline2");
    }

    // T-006: FR-004 — br/ (自己閉じ) を改行に変換
    #[test]
    fn test_br_self_closing_to_newline() {
        assert_eq!(sanitize("line1<br/>line2"), "line1\nline2");
    }

    // T-007: FR-005 — divider を水平線に変換
    #[test]
    fn test_divider_to_hr() {
        assert_eq!(sanitize("<divider/>"), "---");
    }

    // T-008: FR-006 — mention-page をリンクに変換
    #[test]
    fn test_mention_page_to_link() {
        assert_eq!(
            sanitize("<mention-page url=\"https://www.notion.so/abc123\"/>"),
            "[Notion page](https://www.notion.so/abc123)"
        );
    }

    // T-009: FR-007 — mention-user を @user に変換
    #[test]
    fn test_mention_user_to_at() {
        assert_eq!(sanitize("<mention-user url=\"user://abc-123\"/>"), "@user");
    }

    // T-010: FR-008 — file を attachment に変換
    #[test]
    fn test_file_to_attachment() {
        assert_eq!(
            sanitize("<file src=\"file://doc.pdf\"></file>"),
            "[attachment]"
        );
    }

    // T-011: FR-009 — 画像 URL の S3 署名を除去
    #[test]
    fn test_image_url_signature_stripped() {
        let input = "![](https://prod-files-secure.s3.us-west-2.amazonaws.com/img.png?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIA)";
        let expected = "![](https://prod-files-secure.s3.us-west-2.amazonaws.com/img.png)";
        assert_eq!(sanitize(input), expected);
    }

    // T-012: FR-010 — 基本テーブル変換
    #[test]
    fn test_basic_table_conversion() {
        let input = "<table>\n<tr>\n<td>A</td>\n<td>B</td>\n</tr>\n<tr>\n<td>1</td>\n<td>2</td>\n</tr>\n</table>";
        let result = sanitize(input);
        assert!(result.contains("| A | B |"));
        assert!(result.contains("| --- | --- |"));
        assert!(result.contains("| 1 | 2 |"));
    }

    // T-013: FR-011 — テーブルセル内パイプエスケープ
    #[test]
    fn test_table_cell_pipe_escaped() {
        let input = "<table>\n<tr>\n<td>H</td>\n</tr>\n<tr>\n<td>a|b</td>\n</tr>\n</table>";
        let result = sanitize(input);
        assert!(result.contains(r"a\|b"));
    }

    // T-014: FR-012 — テーブルセル内 br 除去
    #[test]
    fn test_table_cell_br_removed() {
        let input = "<table>\n<tr>\n<td>H</td>\n</tr>\n<tr>\n<td>a<br>b</td>\n</tr>\n</table>";
        let result = sanitize(input);
        assert!(result.contains("a b"));
    }

    // T-015: FR-013 — テーブルセル内 span 除去
    #[test]
    fn test_table_cell_span_removed() {
        let input = "<table>\n<tr>\n<td>H</td>\n</tr>\n<tr>\n<td><span color=\"red\">x</span></td>\n</tr>\n</table>";
        let result = sanitize(input);
        assert!(result.contains("| x |"));
    }

    // T-016: FR-010 — テーブル内 colgroup 除去
    #[test]
    fn test_table_colgroup_removed() {
        let input = "<table>\n<colgroup>\n<col>\n<col>\n</colgroup>\n<tr>\n<td>A</td>\n<td>B</td>\n</tr>\n</table>";
        let result = sanitize(input);
        assert!(result.contains("| A | B |"));
        assert!(!result.contains("colgroup"));
    }

    // T-017: FR-010 — テーブルセル属性除去
    #[test]
    fn test_table_cell_attribute_removed() {
        let input = "<table>\n<tr>\n<td color=\"yellow_bg\">text</td>\n</tr>\n</table>";
        let result = sanitize(input);
        assert!(result.contains("| text |"));
    }

    // T-018: FR-010 — 空テーブル
    #[test]
    fn test_empty_table() {
        let result = sanitize("<table>\n</table>");
        assert!(!result.contains("table"));
        assert!(result.trim().is_empty());
    }

    // T-019: FR-010 — 1列テーブル
    #[test]
    fn test_single_column_table() {
        let input = "<table>\n<tr>\n<td>H</td>\n</tr>\n<tr>\n<td>V</td>\n</tr>\n</table>";
        let result = sanitize(input);
        assert!(result.contains("| H |"));
        assert!(result.contains("| --- |"));
        assert!(result.contains("| V |"));
    }

    // T-020: FR-014 — コードブロック内保護
    #[test]
    fn test_code_block_preserved() {
        let input = "before\n```\n{color=\"red\"}\n<empty-block/>\n```\nafter";
        let result = sanitize(input);
        assert!(result.contains("{color=\"red\"}"));
        assert!(result.contains("<empty-block/>"));
    }

    // T-021: FR-015 — インラインコード内保護
    #[test]
    fn test_inline_code_preserved() {
        let input = "use `<empty-block/>` tag";
        let result = sanitize(input);
        assert!(result.contains("`<empty-block/>`"));
    }

    // T-022: FR-016 — 連続空行の正規化
    #[test]
    fn test_consecutive_blank_lines_normalized() {
        assert_eq!(sanitize("a\n\n\n\n\nb"), "a\n\n\nb");
    }

    // T-023: FR-017 — 通常 Markdown 通過
    #[test]
    fn test_normal_markdown_passthrough() {
        let input = "# Title\n\n- list\n\n**bold**";
        assert_eq!(sanitize(input), input);
    }

    // T-024: FR-017 — 不正タグ（閉じタグなし）
    #[test]
    fn test_malformed_tag_survives() {
        let input = "<span color=\"red\">no close";
        let result = sanitize(input);
        assert!(result.contains("no close"));
    }

    // T-025: ALL — 複合パターン
    #[test]
    fn test_composite_pattern() {
        let input = concat!(
            "## Section {color=\"gray_bg\"}\n",
            "<mention-page url=\"https://notion.so/page1\"/> \n",
            "<span color=\"red\">warning</span><br>",
            "<table>\n<tr>\n<td>Col</td>\n</tr>\n<tr>\n<td>val</td>\n</tr>\n</table>\n",
            "![](https://s3.amazonaws.com/img.png?X-Amz-Algorithm=test)",
            "<empty-block/>"
        );
        let result = sanitize(input);
        assert!(!result.contains("{color="));
        assert!(result.contains("[Notion page](https://notion.so/page1)"));
        assert!(result.contains("warning"));
        assert!(!result.contains("<span"));
        assert!(result.contains("| Col |"));
        assert!(!result.contains("X-Amz-Algorithm"));
        assert!(!result.contains("<empty-block/>"));
    }
}
