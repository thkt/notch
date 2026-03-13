const MAX_OUTPUT_BYTES: usize = 100 * 1024;

pub struct FormatResult {
    pub stdout: String,
    pub warnings: Vec<String>,
}

pub fn format_output(title: &str, markdown: &str, truncated_by_api: bool) -> FormatResult {
    let mut warnings = Vec::new();

    if truncated_by_api {
        warnings
            .push("Warning: Page content was truncated by Notion API (page too large)".to_string());
    }

    let mut output = if title.is_empty() {
        markdown.to_string()
    } else {
        format!("# {title}\n\n{markdown}")
    };

    if output.len() > MAX_OUTPUT_BYTES {
        let end = output.floor_char_boundary(MAX_OUTPUT_BYTES);
        output.truncate(end);
        warnings.push("(truncated: output exceeded 100KB)".to_string());
    }

    FormatResult {
        stdout: output,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_with_title() {
        let result = format_output("Test Page", "Hello world", false);
        assert_eq!(result.stdout, "# Test Page\n\nHello world");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_format_empty_title() {
        let result = format_output("", "Hello world", false);
        assert_eq!(result.stdout, "Hello world");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_format_api_truncated_warning() {
        let result = format_output("Title", "body", true);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("truncated by Notion API"));
    }

    #[test]
    fn test_format_output_size_truncation() {
        let large = "a".repeat(200 * 1024); // 200KB
        let result = format_output("", &large, false);
        assert!(result.stdout.len() <= MAX_OUTPUT_BYTES);
        assert!(result.warnings.iter().any(|w| w.contains("exceeded 100KB")));
    }

    #[test]
    fn test_format_output_truncation_respects_utf8_boundary() {
        let padding = "a".repeat(MAX_OUTPUT_BYTES - 10);
        let input = format!("{padding}あいうえお"); // 境界付近にマルチバイト
        let result = format_output("", &input, false);
        assert!(result.stdout.len() <= MAX_OUTPUT_BYTES);
    }
}
