use crate::sanitize::sanitize;

const MAX_OUTPUT_BYTES: usize = 100 * 1024;

pub struct FormatResult {
    pub stdout: String,
    pub warnings: Vec<String>,
}

pub fn format_output(title: &str, markdown: &str, truncated_by_api: bool) -> FormatResult {
    let mut warnings = Vec::new();

    if truncated_by_api {
        warnings
            .push("Warning: Page content was truncated by Notion API (page too large)".to_owned());
    }

    let sanitized = sanitize(markdown);

    let mut output = if title.is_empty() {
        sanitized
    } else {
        format!("# {title}\n\n{sanitized}")
    };

    if output.len() > MAX_OUTPUT_BYTES {
        let end = output.floor_char_boundary(MAX_OUTPUT_BYTES);
        output.truncate(end);
        warnings.push("(truncated: output exceeded 100KB)".to_owned());
    }

    FormatResult {
        stdout: output,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T-036: format_output — タイトルあり
    #[test]
    fn test_format_with_title() {
        let result = format_output("Test Page", "Hello world", false);
        assert_eq!(result.stdout, "# Test Page\n\nHello world");
        assert!(
            result.warnings.is_empty(),
            "expected no warnings, got: {:?}",
            result.warnings
        );
    }

    // T-037: format_output — タイトルなし（空文字列）
    #[test]
    fn test_format_empty_title() {
        let result = format_output("", "Hello world", false);
        assert_eq!(result.stdout, "Hello world");
        assert!(
            result.warnings.is_empty(),
            "expected no warnings, got: {:?}",
            result.warnings
        );
    }

    // T-038: format_output — API truncated フラグが警告メッセージになる
    #[test]
    fn test_format_api_truncated_warning() {
        let result = format_output("Title", "body", true);
        assert_eq!(result.warnings.len(), 1);
        assert!(
            result.warnings[0].contains("truncated by Notion API"),
            "got: {}",
            result.warnings[0]
        );
    }

    // T-039: format_output — 100KB 超のとき切り詰め警告を出す
    #[test]
    fn test_format_output_size_truncation() {
        let large = "a".repeat(200 * 1024); // 200KB
        let result = format_output("", &large, false);
        assert!(result.stdout.len() <= MAX_OUTPUT_BYTES);
        assert!(result.warnings.iter().any(|w| w.contains("exceeded 100KB")));
    }

    // T-040: format_output — 切り詰めが UTF-8 文字境界を尊重する
    #[test]
    fn test_format_output_truncation_respects_utf8_boundary() {
        let padding = "a".repeat(MAX_OUTPUT_BYTES - 10);
        let input = format!("{padding}あいうえお"); // 境界付近にマルチバイト
        let result = format_output("", &input, false);
        assert!(result.stdout.len() <= MAX_OUTPUT_BYTES);
    }

    // T-026: FR-018 — sanitize は body のみに適用される
    #[test]
    fn test_sanitize_applies_to_body_only() {
        let result = format_output("Title {color=\"gray\"}", "<empty-block/>content", false);
        // Title should still contain {color=...} — not sanitized
        assert!(
            result.stdout.contains("{color=\"gray\"}"),
            "got: {}",
            result.stdout
        );
        // Body should be sanitized — <empty-block/> removed
        assert!(
            !result.stdout.contains("<empty-block/>"),
            "got: {}",
            result.stdout
        );
    }
}
