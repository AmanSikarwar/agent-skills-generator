//! Content processor module for the agent-skills-generator.
//!
//! This module handles the core processing logic:
//! 1. Cleaning HTML by removing noise elements (nav, footer, scripts, etc.)
//! 2. Extracting metadata (title, description)
//! 3. Converting HTML to Markdown
//! 4. Generating the consolidated SKILL.md file
//!
//! ## Consolidated Output
//!
//! All content is written to a single `SKILL.md` file containing:
//! - YAML frontmatter with metadata
//! - Page title
//! - Full converted markdown content

use crate::config::Config;
use crate::utils::{extract_url_path, sanitize_skill_name, truncate_description};
use anyhow::{Context, Result};
use chrono::Utc;
use htmd::HtmlToMarkdown;
use scraper::{Html, Selector};
use std::path::Path;
use tracing::{debug, warn};

/// Maximum description length in frontmatter.
const MAX_DESCRIPTION_LENGTH: usize = 1024;

/// Character threshold for large content warning.
/// ~20,000 characters is roughly 5,000 tokens.
const LARGE_CONTENT_THRESHOLD: usize = 20_000;

/// Metadata extracted from a page.
#[derive(Debug, Clone)]
pub struct PageMetadata {
    /// Page title from <title> or <h1> element.
    pub title: String,

    /// Meta description from <meta name="description">.
    pub description: String,

    /// Original URL of the page.
    pub url: String,

    /// Sanitized skill name (kebab-case, max 64 chars).
    pub skill_name: String,

    /// Timestamp when the page was processed.
    pub processed_at: String,
}

/// Result of processing a page.
#[derive(Debug)]
pub struct ProcessedPage {
    /// Metadata extracted from the page.
    pub metadata: PageMetadata,

    /// Cleaned HTML content.
    pub cleaned_html: String,

    /// Markdown-converted content.
    pub markdown_content: String,

    /// Generated SKILL.md content (includes full markdown).
    pub skill_md: String,
}

/// Content processor that cleans HTML and generates skill files.
pub struct Processor {
    /// CSS selectors for elements to remove.
    /// Currently using regex-based removal for better control, but these
    /// selectors are available for future DOM-based implementations.
    #[allow(dead_code)]
    remove_selectors: Vec<Selector>,

    /// HTML to Markdown converter.
    converter: HtmlToMarkdown,
}

impl Processor {
    /// Creates a new processor with the given configuration.
    pub fn new(config: &Config) -> Result<Self> {
        let mut remove_selectors = Vec::new();

        for selector_str in &config.remove_selectors {
            match Selector::parse(selector_str) {
                Ok(selector) => remove_selectors.push(selector),
                Err(e) => {
                    warn!(
                        "Failed to parse CSS selector '{}': {:?}. Skipping.",
                        selector_str, e
                    );
                }
            }
        }

        let converter = HtmlToMarkdown::new();

        Ok(Self {
            remove_selectors,
            converter,
        })
    }

    /// Processes a page: cleans HTML, extracts metadata, generates skill file.
    ///
    /// # Arguments
    /// * `url` - The original URL of the page
    /// * `html` - The raw HTML content
    ///
    /// # Returns
    /// A `ProcessedPage` containing all generated content.
    pub fn process(&self, url: &str, html: &str) -> Result<ProcessedPage> {
        // Step 1: Parse HTML
        let document = Html::parse_document(html);

        // Step 2: Extract metadata before cleaning
        let metadata = self.extract_metadata(url, &document)?;

        // Step 3: Clean HTML by removing noise elements
        let cleaned_html = self.clean_html(html)?;

        // Step 4: Convert to Markdown
        let raw_markdown = self
            .converter
            .convert(&cleaned_html)
            .with_context(|| format!("Failed to convert HTML to markdown for: {}", url))?;

        // Step 5: Post-process markdown to remove remaining artifacts
        let markdown_content = self.clean_markdown(&raw_markdown);

        // Step 6: Generate consolidated SKILL.md content with full markdown
        let skill_md = self.generate_skill_md(&metadata, &markdown_content);

        Ok(ProcessedPage {
            metadata,
            cleaned_html,
            markdown_content,
            skill_md,
        })
    }

    /// Extracts metadata from the parsed HTML document.
    fn extract_metadata(&self, url: &str, document: &Html) -> Result<PageMetadata> {
        // Extract title
        let title = self
            .extract_title(document)
            .unwrap_or_else(|| "Untitled".to_string());

        // Extract meta description
        let description = self.extract_meta_description(document).unwrap_or_else(|| {
            // Fall back to first paragraph if no meta description
            self.extract_first_paragraph(document).unwrap_or_default()
        });

        // Generate skill name from URL path
        let url_path = extract_url_path(url);
        let skill_name = sanitize_skill_name(&url_path);

        // Handle edge case where skill_name is empty (e.g., root URL)
        let skill_name = if skill_name.is_empty() {
            // Use domain as skill name
            crate::utils::extract_domain(url)
                .map(|d| sanitize_skill_name(&d))
                .unwrap_or_else(|| "index".to_string())
        } else {
            skill_name
        };

        Ok(PageMetadata {
            title,
            description,
            url: url.to_string(),
            skill_name,
            processed_at: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        })
    }

    /// Extracts the page title.
    fn extract_title(&self, document: &Html) -> Option<String> {
        // Try <title> first
        if let Ok(selector) = Selector::parse("title")
            && let Some(element) = document.select(&selector).next()
        {
            let title: String = element.text().collect();
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }

        // Fall back to first <h1>
        if let Ok(selector) = Selector::parse("h1")
            && let Some(element) = document.select(&selector).next()
        {
            let title: String = element.text().collect();
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }

        None
    }

    /// Extracts the meta description.
    fn extract_meta_description(&self, document: &Html) -> Option<String> {
        if let Ok(selector) = Selector::parse("meta[name='description']")
            && let Some(element) = document.select(&selector).next()
            && let Some(content) = element.value().attr("content")
        {
            let content = content.trim();
            if !content.is_empty() {
                return Some(content.to_string());
            }
        }

        // Try og:description as fallback
        if let Ok(selector) = Selector::parse("meta[property='og:description']")
            && let Some(element) = document.select(&selector).next()
            && let Some(content) = element.value().attr("content")
        {
            let content = content.trim();
            if !content.is_empty() {
                return Some(content.to_string());
            }
        }

        None
    }

    /// Extracts the first paragraph as a fallback description.
    fn extract_first_paragraph(&self, document: &Html) -> Option<String> {
        if let Ok(selector) = Selector::parse("p") {
            for element in document.select(&selector) {
                let text: String = element.text().collect();
                let text = text.trim();
                if text.len() > 50 {
                    // Only use if it's substantial
                    return Some(truncate_description(text, 200));
                }
            }
        }
        None
    }

    /// Cleans HTML by removing noise elements.
    ///
    /// This is critical for token optimization - we remove:
    /// - Navigation elements (<nav>)
    /// - Footers (<footer>)
    /// - Scripts and styles
    /// - Sidebars and menus
    /// - Table of contents
    /// - Ads and cookie banners
    /// - Skip links and accessibility shortcuts
    /// - Material icons and icon fonts
    fn clean_html(&self, html: &str) -> Result<String> {
        // We'll use regex patterns to remove noise elements from HTML.
        // Note: For production, consider using a proper HTML manipulation library
        // but regex works well for removing well-structured noise elements.

        let mut cleaned = html.to_string();

        // Remove script tags and their content
        let script_re = regex::Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
        cleaned = script_re.replace_all(&cleaned, "").to_string();

        // Remove style tags and their content
        let style_re = regex::Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
        cleaned = style_re.replace_all(&cleaned, "").to_string();

        // Remove noscript tags and their content
        let noscript_re = regex::Regex::new(r"(?is)<noscript[^>]*>.*?</noscript>").unwrap();
        cleaned = noscript_re.replace_all(&cleaned, "").to_string();

        // Remove template tags (often used for dynamic content)
        let template_re = regex::Regex::new(r"(?is)<template[^>]*>.*?</template>").unwrap();
        cleaned = template_re.replace_all(&cleaned, "").to_string();

        // Remove common noise elements by tag name
        let noise_patterns = [
            r"(?is)<nav[^>]*>.*?</nav>",
            r"(?is)<footer[^>]*>.*?</footer>",
            r"(?is)<header[^>]*>.*?</header>",
            r"(?is)<aside[^>]*>.*?</aside>",
            r"(?is)<iframe[^>]*>.*?</iframe>",
            r"(?is)<svg[^>]*>.*?</svg>",
            r"(?is)<canvas[^>]*>.*?</canvas>",
            r"(?is)<video[^>]*>.*?</video>",
            r"(?is)<audio[^>]*>.*?</audio>",
            r"(?is)<form[^>]*>.*?</form>", // Remove forms (search, feedback, etc.)
        ];

        for pattern in noise_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Remove elements by ID patterns (common noise IDs)
        let id_patterns = [
            r#"(?is)<[^>]+id="[^"]*\b(cookie|consent|banner|popup|modal|overlay|gdpr|privacy-notice|skip-link|feedback|newsletter|subscribe)\b[^"]*"[^>]*>.*?</[^>]+>"#,
        ];

        for pattern in id_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Remove elements with common noise class names
        let class_patterns = [
            // Navigation and menus
            r#"(?is)<[^>]+class="[^"]*\b(nav|navigation|menu|sidebar|toc|table-of-contents|breadcrumb|breadcrumbs)\b[^"]*"[^>]*>.*?</[^>]+>"#,
            // Cookie and consent banners
            r#"(?is)<[^>]+class="[^"]*\b(cookie|consent|gdpr|privacy-notice|cookie-banner|cookie-consent)\b[^"]*"[^>]*>.*?</[^>]+>"#,
            // Ads and promotional content
            r#"(?is)<[^>]+class="[^"]*\b(ads?|advertisement|promo|promotional|banner|announcement)\b[^"]*"[^>]*>.*?</[^>]+>"#,
            // Feedback and ratings
            r#"(?is)<[^>]+class="[^"]*\b(feedback|rating|ratings|helpful|thumbs|vote|voting)\b[^"]*"[^>]*>.*?</[^>]+>"#,
            // Skip links and accessibility shortcuts
            r#"(?is)<[^>]+class="[^"]*\b(skip-link|skip-to-content|sr-only|visually-hidden)\b[^"]*"[^>]*>.*?</[^>]+>"#,
            // Social sharing
            r#"(?is)<[^>]+class="[^"]*\b(social|share|sharing|follow-us)\b[^"]*"[^>]*>.*?</[^>]+>"#,
            // Page metadata/footer info
            r#"(?is)<[^>]+class="[^"]*\b(page-meta|page-info|last-updated|edit-page|view-source|report-issue)\b[^"]*"[^>]*>.*?</[^>]+>"#,
        ];

        for pattern in class_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Remove skip links (often standalone anchor tags)
        // Using r##""## because the pattern contains # character
        if let Ok(skip_link_re) =
            regex::Regex::new(r##"(?is)<a[^>]+href="#[^"]*"[^>]*>Skip[^<]*</a>"##)
        {
            cleaned = skip_link_re.replace_all(&cleaned, "").to_string();
        }

        // Remove material icons and icon fonts (span/i elements with icon classes)
        let icon_patterns = [
            r"(?is)<span[^>]+class=[^>]*(material-icons|icon|fa|fas|far|fab|glyphicon)[^>]*>[^<]*</span>",
            r"(?is)<i[^>]+class=[^>]*(material-icons|icon|fa|fas|far|fab|glyphicon)[^>]*>[^<]*</i>",
            // Remove inline material icon text that might be in any element
            r"(?is)<[^>]+class=[^>]*material-symbols[^>]*>[^<]*</[^>]+>",
        ];

        for pattern in icon_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Remove button elements that are likely UI controls (copy buttons, etc.)
        if let Ok(button_re) = regex::Regex::new(r"(?is)<button[^>]*>.*?</button>") {
            cleaned = button_re.replace_all(&cleaned, "").to_string();
        }

        // Remove HTML comments
        let comment_re = regex::Regex::new(r"(?s)<!--.*?-->").unwrap();
        cleaned = comment_re.replace_all(&cleaned, "").to_string();

        // Remove data attributes that might contain noise
        if let Ok(data_attr_re) = regex::Regex::new(r#"\s+data-[a-z-]+="[^"]*""#) {
            cleaned = data_attr_re.replace_all(&cleaned, "").to_string();
        }

        debug!("Cleaned HTML: {} -> {} bytes", html.len(), cleaned.len());

        Ok(cleaned)
    }

    /// Post-processes markdown to remove remaining noise artifacts.
    ///
    /// This handles content that slips through HTML cleaning, such as:
    /// - Material icon names (chevron_right, content_copy, etc.)
    /// - Skip link text
    /// - Cookie consent text
    /// - Feedback prompts
    /// - Page metadata footers
    fn clean_markdown(&self, markdown: &str) -> String {
        let mut cleaned = markdown.to_string();

        // Remove common material icon names that appear as text
        let icon_names = [
            "chevron_right",
            "chevron_left",
            "arrow_forward",
            "arrow_back",
            "arrow_drop_down",
            "arrow_drop_up",
            "content_copy",
            "content_paste",
            "thumb_up",
            "thumb_down",
            "thumbs_up",
            "thumbs_down",
            "vertical_align_top",
            "vertical_align_bottom",
            "expand_more",
            "expand_less",
            "menu",
            "close",
            "search",
            "home",
            "settings",
            "check",
            "check_circle",
            "error",
            "warning",
            "info",
            "list",
            "share",
            "edit",
            "delete",
            "add",
            "remove",
            "star",
            "star_border",
            "favorite",
            "favorite_border",
            "bookmark",
            "bookmark_border",
            "visibility",
            "visibility_off",
            "lock",
            "lock_open",
            "person",
            "people",
            "notifications",
            "email",
            "phone",
            "location_on",
            "calendar_today",
            "schedule",
            "more_vert",
            "more_horiz",
            "open_in_new",
            "launch",
            "link",
            "file_download",
            "file_upload",
            "cloud_download",
            "cloud_upload",
            "play_arrow",
            "pause",
            "stop",
            "skip_next",
            "skip_previous",
            "fast_forward",
            "fast_rewind",
            "volume_up",
            "volume_down",
            "volume_mute",
            "fullscreen",
            "fullscreen_exit",
            "zoom_in",
            "zoom_out",
            "refresh",
            "sync",
            "cached",
            "done",
            "done_all",
            "clear",
            "cancel",
            "help",
            "help_outline",
            "code",
        ];

        for icon in icon_names {
            // Remove icon name with word boundaries (handles inline occurrences)
            // This catches icons appearing anywhere in text
            let pattern = format!(r"\b{}\b", regex::escape(icon));
            if let Ok(re) = regex::Regex::new(&pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Clean up lines that become empty or only whitespace after icon removal
        let empty_lines_re = regex::Regex::new(r"(?m)^\s*$").unwrap();
        cleaned = empty_lines_re.replace_all(&cleaned, "").to_string();

        // Remove skip link patterns
        let skip_patterns = [
            r"(?m)^\[Skip to main content\]\([^)]*\)\s*$",
            r"(?m)^\[Skip to content\]\([^)]*\)\s*$",
            r"(?m)^Skip to (main )?content\s*$",
        ];

        for pattern in skip_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Remove cookie consent notices
        let cookie_patterns = [
            r"(?is).*uses cookies.*\n.*Learn more.*OK,? got it\s*",
            r"(?is)This site uses cookies.*\n?.*Accept\s*",
            r"(?is)We use cookies.*\n?.*Got it\s*",
        ];

        for pattern in cookie_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Remove feedback prompts
        let feedback_patterns = [
            r"(?m)^Was this page'?s? content helpful\?\s*$",
            r"(?m)^Was this helpful\?\s*$",
            r"(?m)^Did you find this helpful\?\s*$",
            r"(?m)^Rate this page:?\s*$",
        ];

        for pattern in feedback_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Remove page metadata footers (common patterns)
        let footer_patterns = [
            r"(?m)^Unless stated otherwise.*Page last updated.*$",
            r"(?m)^Page last updated on \d{4}-\d{1,2}-\d{1,2}\.?\s*$",
            r"(?m)^\[View source\]\([^)]*\).*\[report an issue\]\([^)]*\).*$",
            r"(?m)^Last modified:.*$",
            r"(?m)^Last updated:.*$",
        ];

        for pattern in footer_patterns {
            if let Ok(re) = regex::Regex::new(&format!("(?i){}", pattern)) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Remove promotional banners (Check out our...)
        let promo_patterns = [
            r"(?m)^Check out our newly published.*$",
            r"(?m)^🎉.*new.*!?\s*$",
            r"(?m)^📢.*announcement.*$",
        ];

        for pattern in promo_patterns {
            if let Ok(re) = regex::Regex::new(&format!("(?i){}", pattern)) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }

        // Clean up excessive blank lines (more than 2 consecutive)
        let blank_lines_re = regex::Regex::new(r"\n{4,}").unwrap();
        cleaned = blank_lines_re.replace_all(&cleaned, "\n\n\n").to_string();

        // Clean up lines that only contain whitespace
        let whitespace_lines_re = regex::Regex::new(r"(?m)^\s+$").unwrap();
        cleaned = whitespace_lines_re.replace_all(&cleaned, "").to_string();

        cleaned.trim().to_string()
    }

    /// Generates the consolidated SKILL.md content with full markdown.
    ///
    /// The SKILL.md file now contains ALL content directly:
    /// - YAML frontmatter with metadata
    /// - Page title
    /// - Full converted markdown content
    ///
    /// This simplifies the output structure to a single file per skill.
    fn generate_skill_md(&self, metadata: &PageMetadata, markdown_content: &str) -> String {
        let truncated_description =
            truncate_description(&metadata.description, MAX_DESCRIPTION_LENGTH);

        // Warn if content is large (may consume many tokens)
        let total_chars = markdown_content.len();
        if total_chars > LARGE_CONTENT_THRESHOLD {
            warn!(
                "Large skill '{}': {} characters (~{} tokens). Consider splitting into smaller sections.",
                metadata.skill_name,
                total_chars,
                total_chars / 4 // Rough token estimate
            );
        }

        format!(
            r#"---
name: {name}
description: {description}
metadata:
  url: {url}
---

# {title}

{content}
"#,
            name = metadata.skill_name,
            description = truncated_description.replace('\n', " ").replace('\r', ""),
            url = metadata.url,
            title = metadata.title,
            content = markdown_content.trim(),
        )
    }

    /// Writes the processed page to the output directory.
    ///
    /// Creates the following structure:
    /// ```text
    /// output_dir/
    ///   skill-name/
    ///     SKILL.md  <-- Contains ALL content
    /// ```
    pub async fn write_to_disk(
        &self,
        processed: &ProcessedPage,
        output_dir: &Path,
    ) -> Result<std::path::PathBuf> {
        use fs_err::tokio as fs;

        // Create skill directory
        let skill_dir = output_dir.join(&processed.metadata.skill_name);
        fs::create_dir_all(&skill_dir).await.with_context(|| {
            format!("Failed to create skill directory: {}", skill_dir.display())
        })?;

        // Write SKILL.md with full content
        let skill_md_path = skill_dir.join("SKILL.md");
        fs::write(&skill_md_path, &processed.skill_md)
            .await
            .with_context(|| format!("Failed to write SKILL.md: {}", skill_md_path.display()))?;

        debug!(
            "Wrote skill '{}' ({} chars) to {}",
            processed.metadata.skill_name,
            processed.skill_md.len(),
            skill_dir.display()
        );

        Ok(skill_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_extract_metadata() {
        let processor = Processor::new(&test_config()).unwrap();

        let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Test Page Title</title>
    <meta name="description" content="This is a test description.">
</head>
<body>
    <h1>Main Heading</h1>
    <p>Some content here.</p>
</body>
</html>
"#;

        let document = Html::parse_document(html);
        let metadata = processor
            .extract_metadata("https://example.com/docs/test", &document)
            .unwrap();

        assert_eq!(metadata.title, "Test Page Title");
        assert_eq!(metadata.description, "This is a test description.");
        assert_eq!(metadata.skill_name, "docs-test");
    }

    #[test]
    fn test_clean_html() {
        let processor = Processor::new(&test_config()).unwrap();

        let html = r#"
<!DOCTYPE html>
<html>
<head>
    <script>console.log("remove me");</script>
    <style>.foo { color: red; }</style>
</head>
<body>
    <nav>Navigation</nav>
    <main>
        <h1>Keep This</h1>
        <p>Important content.</p>
    </main>
    <footer>Footer content</footer>
</body>
</html>
"#;

        let cleaned = processor.clean_html(html).unwrap();

        assert!(!cleaned.contains("<script>"));
        assert!(!cleaned.contains("<style>"));
        assert!(!cleaned.contains("<nav>"));
        assert!(!cleaned.contains("<footer>"));
        assert!(cleaned.contains("Keep This"));
        assert!(cleaned.contains("Important content"));
    }

    #[test]
    fn test_generate_skill_md_contains_full_content() {
        let processor = Processor::new(&test_config()).unwrap();

        let metadata = PageMetadata {
            title: "Flutter Installation Guide".to_string(),
            description: "Learn how to install Flutter on your system.".to_string(),
            url: "https://docs.flutter.dev/get-started/install".to_string(),
            skill_name: "get-started-install".to_string(),
            processed_at: "2024-01-15T10:30:00Z".to_string(),
        };

        let markdown_content =
            "## Installation Steps\n\n1. Download Flutter\n2. Extract the archive\n3. Add to PATH";
        let skill_md = processor.generate_skill_md(&metadata, markdown_content);

        // Check frontmatter
        assert!(skill_md.contains("name: get-started-install"));
        assert!(skill_md.contains("url: https://docs.flutter.dev/get-started/install"));

        // Check title
        assert!(skill_md.contains("# Flutter Installation Guide"));

        // Check that FULL content is included (not a reference link)
        assert!(skill_md.contains("## Installation Steps"));
        assert!(skill_md.contains("1. Download Flutter"));
        assert!(skill_md.contains("2. Extract the archive"));
        assert!(skill_md.contains("3. Add to PATH"));

        // Verify NO reference pattern artifacts
        assert!(!skill_md.contains("references/content.md"));
        assert!(!skill_md.contains("[View Documentation]"));
    }

    #[test]
    fn test_process_page() {
        let processor = Processor::new(&test_config()).unwrap();

        let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>API Reference</title>
    <meta name="description" content="Complete API documentation.">
</head>
<body>
    <main>
        <h1>API Reference</h1>
        <p>This is the main content.</p>
        <h2>Methods</h2>
        <p>Method documentation here.</p>
    </main>
</body>
</html>
"#;

        let processed = processor
            .process("https://example.com/docs/api", html)
            .unwrap();

        assert_eq!(processed.metadata.title, "API Reference");
        assert_eq!(processed.metadata.skill_name, "docs-api");

        // SKILL.md should contain the full content
        assert!(processed.skill_md.contains("name: docs-api"));
        assert!(processed.skill_md.contains("# API Reference"));
        assert!(processed.skill_md.contains("Methods"));
        assert!(processed.skill_md.contains("Method documentation here"));

        // No reference pattern
        assert!(!processed.skill_md.contains("references/"));
    }

    #[test]
    fn test_clean_markdown_removes_icon_names() {
        let processor = Processor::new(&test_config()).unwrap();

        let markdown = r#"
list chevron_right

# Main Title

content_copy

Some actual content here.

thumb_up thumb_down

Was this page's content helpful?
"#;

        let cleaned = processor.clean_markdown(markdown);

        // Icon names should be removed
        assert!(!cleaned.contains("chevron_right"));
        assert!(!cleaned.contains("content_copy"));
        assert!(!cleaned.contains("thumb_up"));
        assert!(!cleaned.contains("thumb_down"));

        // Actual content should remain
        assert!(cleaned.contains("# Main Title"));
        assert!(cleaned.contains("Some actual content here"));

        // Feedback prompt should be removed
        assert!(!cleaned.contains("Was this page's content helpful"));
    }

    #[test]
    fn test_clean_markdown_removes_skip_links() {
        let processor = Processor::new(&test_config()).unwrap();

        let markdown = r#"
[Skip to main content](#site-content)

# Welcome

This is the main content.
"#;

        let cleaned = processor.clean_markdown(markdown);

        assert!(!cleaned.contains("Skip to main content"));
        assert!(cleaned.contains("# Welcome"));
        assert!(cleaned.contains("This is the main content"));
    }

    #[test]
    fn test_clean_markdown_removes_cookie_notice() {
        let processor = Processor::new(&test_config()).unwrap();

        let markdown = r#"
example.com uses cookies from Google to deliver and enhance the quality of its services.

Learn more OK, got it

# Main Content

Actual page content here.
"#;

        let cleaned = processor.clean_markdown(markdown);

        assert!(!cleaned.contains("uses cookies"));
        assert!(!cleaned.contains("OK, got it"));
        assert!(cleaned.contains("# Main Content"));
        assert!(cleaned.contains("Actual page content here"));
    }

    #[test]
    fn test_clean_markdown_removes_page_footer() {
        let processor = Processor::new(&test_config()).unwrap();

        let markdown = r#"
# Documentation

Content here.

Unless stated otherwise, the documentation on this site reflects Flutter 3.38.6. Page last updated on 2025-10-28.
"#;

        let cleaned = processor.clean_markdown(markdown);

        assert!(!cleaned.contains("Unless stated otherwise"));
        assert!(!cleaned.contains("Page last updated"));
        assert!(cleaned.contains("# Documentation"));
        assert!(cleaned.contains("Content here"));
    }

    #[test]
    fn test_clean_html_removes_buttons() {
        let processor = Processor::new(&test_config()).unwrap();

        let html = r#"
<div>
    <h1>Code Example</h1>
    <pre><code>print("hello")</code></pre>
    <button class="copy-btn">Copy</button>
</div>
"#;

        let cleaned = processor.clean_html(html).unwrap();

        assert!(!cleaned.contains("<button"));
        assert!(cleaned.contains("Code Example"));
        assert!(cleaned.contains("print"));
    }

    #[test]
    fn test_clean_html_removes_cookie_banner() {
        let processor = Processor::new(&test_config()).unwrap();

        let html = r#"
<div class="cookie-consent">
    <p>This site uses cookies</p>
    <button>Accept</button>
</div>
<main>
    <h1>Welcome</h1>
    <p>Main content</p>
</main>
"#;

        let cleaned = processor.clean_html(html).unwrap();

        assert!(!cleaned.contains("cookie-consent"));
        assert!(!cleaned.contains("This site uses cookies"));
        assert!(cleaned.contains("Welcome"));
        assert!(cleaned.contains("Main content"));
    }
}
