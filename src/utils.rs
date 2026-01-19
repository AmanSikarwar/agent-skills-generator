//! Utility functions for the agent-skills-generator.
//!
//! This module provides helper functions for string sanitization,
//! path manipulation, and other common operations used throughout the crate.

use regex::Regex;
use std::sync::LazyLock;

/// Maximum length for skill names (strict compliance requirement).
const MAX_SKILL_NAME_LENGTH: usize = 64;

/// Pre-compiled regex patterns for sanitization.
/// Using LazyLock for thread-safe, one-time initialization.
static MULTIPLE_HYPHENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-+").expect("Failed to compile multiple hyphens regex"));

static INVALID_CHARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^a-z0-9-]").expect("Failed to compile invalid chars regex"));

static LEADING_TRAILING_HYPHENS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^-+|-+$").expect("Failed to compile leading/trailing hyphens regex")
});

/// Sanitizes a URL path or string into a strict kebab-case skill name.
///
/// # Rules Applied:
/// - Converts to lowercase
/// - Replaces `/` with `-`
/// - Replaces `_` with `-`
/// - Removes dots and other invalid characters
/// - Collapses multiple consecutive hyphens into one
/// - Removes leading/trailing hyphens
/// - Truncates to maximum 64 characters
///
/// # Arguments
/// * `path` - The URL path or string to sanitize
///
/// # Returns
/// A sanitized string suitable for use as a skill directory name.
///
/// # Examples
/// ```
/// use agent_skills_generator::utils::sanitize_skill_name;
///
/// assert_eq!(sanitize_skill_name("foo/bar_baz.html"), "foo-bar-baz");
/// assert_eq!(sanitize_skill_name("/docs/flutter/install"), "docs-flutter-install");
/// assert_eq!(sanitize_skill_name("API_Reference.html"), "api-reference");
/// ```
pub fn sanitize_skill_name(path: &str) -> String {
    // Step 1: Decode any URL-encoded characters and convert to lowercase
    let decoded = urlencoding_decode(path).to_lowercase();

    // Step 2: Replace path separators and underscores with hyphens
    let with_hyphens = decoded.replace(['/', '\\', '_'], "-");

    // Step 3: Remove file extensions (e.g., .html, .htm, .md)
    let without_extension = remove_file_extension(&with_hyphens);

    // Step 4: Remove any characters that aren't alphanumeric or hyphens
    let clean = INVALID_CHARS.replace_all(&without_extension, "");

    // Step 5: Collapse multiple consecutive hyphens into a single hyphen
    let collapsed = MULTIPLE_HYPHENS.replace_all(&clean, "-");

    // Step 6: Remove leading and trailing hyphens
    let trimmed = LEADING_TRAILING_HYPHENS.replace_all(&collapsed, "");

    // Step 7: Truncate to maximum length while respecting word boundaries
    truncate_at_word_boundary(&trimmed, MAX_SKILL_NAME_LENGTH)
}

/// Removes common file extensions from a string.
fn remove_file_extension(s: &str) -> String {
    let extensions = [
        ".html", ".htm", ".md", ".txt", ".php", ".asp", ".aspx", ".jsp",
    ];
    let mut result = s.to_string();
    for ext in extensions {
        if result.ends_with(ext) {
            result = result[..result.len() - ext.len()].to_string();
            break;
        }
    }
    result
}

/// Simple URL decoding for common encoded characters.
fn urlencoding_decode(s: &str) -> String {
    s.replace("%20", " ")
        .replace("%2F", "/")
        .replace("%3A", ":")
        .replace("%3F", "?")
        .replace("%3D", "=")
        .replace("%26", "&")
        .replace("%23", "#")
}

/// Truncates a string at a word (hyphen) boundary if possible.
fn truncate_at_word_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }

    // Find the last hyphen before max_len
    let truncated = &s[..max_len];
    if let Some(last_hyphen) = truncated.rfind('-') {
        // Only use the hyphen boundary if it's reasonably close to max_len
        if last_hyphen > max_len / 2 {
            return truncated[..last_hyphen].to_string();
        }
    }

    // Fall back to hard truncation
    truncated.to_string()
}

/// Extracts the path portion from a URL, removing the domain and query string.
///
/// # Examples
/// ```
/// use agent_skills_generator::utils::extract_url_path;
///
/// assert_eq!(extract_url_path("https://example.com/docs/api"), "/docs/api");
/// assert_eq!(extract_url_path("https://example.com/page?query=1"), "/page");
/// ```
pub fn extract_url_path(url_str: &str) -> String {
    use url::Url;

    match Url::parse(url_str) {
        Ok(url) => {
            let path = url.path().to_string();
            // Return "/" if path is empty
            if path.is_empty() {
                "/".to_string()
            } else {
                path
            }
        }
        Err(_) => {
            // If URL parsing fails, try to extract path manually
            if let Some(start) = url_str.find("://") {
                let after_protocol = &url_str[start + 3..];
                if let Some(slash_pos) = after_protocol.find('/') {
                    let path_and_query = &after_protocol[slash_pos..];
                    // Remove query string
                    if let Some(query_pos) = path_and_query.find('?') {
                        return path_and_query[..query_pos].to_string();
                    }
                    return path_and_query.to_string();
                }
            }
            "/".to_string()
        }
    }
}

/// Truncates a description to fit within token limits.
///
/// This is part of the **Reference Pattern** - we keep SKILL.md lightweight
/// (< 1024 chars for description) to minimize token usage when skills are
/// loaded into LLM context.
///
/// # Arguments
/// * `description` - The full description text
/// * `max_chars` - Maximum character limit (default 1024)
///
/// # Returns
/// A truncated description that ends at a sentence boundary if possible.
pub fn truncate_description(description: &str, max_chars: usize) -> String {
    if description.len() <= max_chars {
        return description.to_string();
    }

    let truncated = &description[..max_chars];

    // Try to find the last sentence boundary
    let sentence_endings = [". ", "! ", "? "];
    let mut best_end = 0;

    for ending in sentence_endings {
        if let Some(pos) = truncated.rfind(ending)
            && pos > best_end
        {
            best_end = pos + 1; // Include the punctuation
        }
    }

    if best_end > max_chars / 2 {
        truncated[..best_end].trim().to_string()
    } else {
        // Fall back to word boundary
        if let Some(last_space) = truncated.rfind(' ') {
            format!("{}...", truncated[..last_space].trim())
        } else {
            format!("{}...", truncated.trim())
        }
    }
}

/// Extracts the domain from a URL.
pub fn extract_domain(url_str: &str) -> Option<String> {
    use url::Url;

    Url::parse(url_str)
        .ok()
        .and_then(|url| url.host_str().map(|s| s.to_string()))
}

/// Parses a URL pattern and extracts the base URL and path pattern.
///
/// # Examples
/// ```
/// use agent_skills_generator::utils::parse_url_pattern;
///
/// let (base, pattern) = parse_url_pattern("https://docs.flutter.dev/ui/*");
/// assert_eq!(base, "https://docs.flutter.dev/ui/");
/// assert_eq!(pattern, Some("https://docs.flutter.dev/ui/*".to_string()));
/// ```
pub fn parse_url_pattern(url: &str) -> (String, Option<String>) {
    // Check if the URL contains a glob pattern
    if url.contains('*') || url.contains('?') {
        // Find where the pattern starts
        let pattern_start = url
            .find('*')
            .unwrap_or(url.len())
            .min(url.find('?').unwrap_or(url.len()));

        // Find the last slash before the pattern
        let base_end = url[..pattern_start]
            .rfind('/')
            .map(|i| i + 1)
            .unwrap_or(pattern_start);

        let base_url = url[..base_end].to_string();

        // Return the base URL and the full pattern for rule matching
        (base_url, Some(url.to_string()))
    } else {
        // No pattern, use URL as-is
        (url.to_string(), None)
    }
}

/// Extracts the domain with protocol from a URL.
///
/// # Examples
/// ```
/// use agent_skills_generator::utils::extract_domain_with_protocol;
///
/// assert_eq!(
///     extract_domain_with_protocol("https://docs.flutter.dev/ui/widgets"),
///     Some("https://docs.flutter.dev".to_string())
/// );
/// ```
pub fn extract_domain_with_protocol(url_str: &str) -> Option<String> {
    use url::Url;

    Url::parse(url_str)
        .ok()
        .map(|url| format!("{}://{}", url.scheme(), url.host_str().unwrap_or("")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_basic() {
        assert_eq!(sanitize_skill_name("foo/bar_baz.html"), "foo-bar-baz");
    }

    #[test]
    fn test_sanitize_with_leading_slash() {
        assert_eq!(
            sanitize_skill_name("/docs/flutter/install"),
            "docs-flutter-install"
        );
    }

    #[test]
    fn test_sanitize_with_underscores() {
        assert_eq!(sanitize_skill_name("API_Reference.html"), "api-reference");
    }

    #[test]
    fn test_sanitize_preserves_numbers() {
        assert_eq!(sanitize_skill_name("v2/api/docs"), "v2-api-docs");
    }

    #[test]
    fn test_sanitize_removes_special_chars() {
        assert_eq!(sanitize_skill_name("foo@bar#baz!"), "foobarbaz");
    }

    #[test]
    fn test_sanitize_collapses_multiple_hyphens() {
        assert_eq!(sanitize_skill_name("foo//bar___baz"), "foo-bar-baz");
    }

    #[test]
    fn test_sanitize_empty_string() {
        assert_eq!(sanitize_skill_name(""), "");
    }

    #[test]
    fn test_sanitize_only_special_chars() {
        assert_eq!(sanitize_skill_name("///___..."), "");
    }

    #[test]
    fn test_sanitize_long_string_truncation() {
        let long_path = "a".repeat(100);
        let result = sanitize_skill_name(&long_path);
        assert!(result.len() <= MAX_SKILL_NAME_LENGTH);
    }

    #[test]
    fn test_sanitize_no_underscores_in_output() {
        let inputs = [
            "hello_world",
            "foo_bar_baz",
            "API_Reference_Guide",
            "some_long_path_with_many_underscores",
        ];

        for input in inputs {
            let result = sanitize_skill_name(input);
            assert!(
                !result.contains('_'),
                "Output '{}' from input '{}' contains underscore",
                result,
                input
            );
        }
    }

    #[test]
    fn test_extract_url_path() {
        assert_eq!(
            extract_url_path("https://example.com/docs/api"),
            "/docs/api"
        );
        assert_eq!(
            extract_url_path("https://example.com/page?query=1"),
            "/page"
        );
        assert_eq!(extract_url_path("https://example.com"), "/");
    }

    #[test]
    fn test_truncate_description() {
        let short = "A short description.";
        assert_eq!(truncate_description(short, 1024), short);

        let long = "A".repeat(2000);
        let result = truncate_description(&long, 100);
        assert!(result.len() <= 103); // 100 + "..."
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://example.com/path"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("https://docs.flutter.dev/guide"),
            Some("docs.flutter.dev".to_string())
        );
    }

    #[test]
    fn test_parse_url_pattern() {
        // URL with wildcard pattern
        let (base, pattern) = parse_url_pattern("https://docs.flutter.dev/ui/*");
        assert_eq!(base, "https://docs.flutter.dev/ui/");
        assert_eq!(pattern, Some("https://docs.flutter.dev/ui/*".to_string()));

        // URL with wildcard in middle
        let (base, pattern) = parse_url_pattern("https://docs.flutter.dev/*/widgets");
        assert_eq!(base, "https://docs.flutter.dev/");
        assert_eq!(
            pattern,
            Some("https://docs.flutter.dev/*/widgets".to_string())
        );

        // URL without pattern
        let (base, pattern) = parse_url_pattern("https://docs.flutter.dev/ui/widgets");
        assert_eq!(base, "https://docs.flutter.dev/ui/widgets");
        assert_eq!(pattern, None);

        // URL with question mark pattern
        let (base, pattern) = parse_url_pattern("https://example.com/v?/api");
        assert_eq!(base, "https://example.com/");
        assert_eq!(pattern, Some("https://example.com/v?/api".to_string()));
    }

    #[test]
    fn test_extract_domain_with_protocol() {
        assert_eq!(
            extract_domain_with_protocol("https://docs.flutter.dev/ui/widgets"),
            Some("https://docs.flutter.dev".to_string())
        );
        assert_eq!(
            extract_domain_with_protocol("http://example.com/path"),
            Some("http://example.com".to_string())
        );
    }
}
