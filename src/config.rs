//! Configuration module for the agent-skills-generator.
//!
//! This module handles loading and parsing the `skills.yaml` configuration file
//! which defines crawling rules, output directories, and other settings.

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Default output directory for generated skills.
const DEFAULT_OUTPUT_DIR: &str = ".agent/skills";

/// Default crawl delay in milliseconds (polite crawling).
const DEFAULT_DELAY_MS: u64 = 100;

/// Default maximum depth for crawling.
const DEFAULT_MAX_DEPTH: usize = 25;

/// Default request timeout in seconds.
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Target IDE/agent for skills generation.
///
/// Each target has specific directory conventions for project-scoped
/// and user-scoped skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillsTarget {
    /// GitHub Copilot: `.github/skills/` (project), `~/.copilot/skills/` (user)
    #[serde(alias = "copilot")]
    GithubCopilot,
    /// Claude Code: `.claude/skills/` (project), `~/.claude/skills/` (user)
    #[serde(alias = "claude")]
    ClaudeCode,
    /// Cursor: `.cursor/skills/` (project), `~/.cursor/skills/` (user)
    Cursor,
    /// Google Antigravity: `.gemini/skills/` (project), `~/.gemini/skills/` (user)
    #[serde(alias = "gemini")]
    Antigravity,
    /// OpenAI Codex: `.codex/skills/` (project), `~/.codex/skills/` (user)
    #[serde(alias = "codex")]
    OpenAICodex,
    /// OpenCode: `.opencode/skills/` (project), `~/.config/opencode/skills/` (user)
    OpenCode,
    /// Custom output path (uses the `output` field directly)
    #[default]
    Custom,
}

impl SkillsTarget {
    /// Returns the project-scoped output directory for this target.
    pub fn project_dir(&self) -> &'static str {
        match self {
            Self::GithubCopilot => ".github/skills",
            Self::ClaudeCode => ".claude/skills",
            Self::Cursor => ".cursor/skills",
            Self::Antigravity => ".gemini/skills",
            Self::OpenAICodex => ".codex/skills",
            Self::OpenCode => ".opencode/skills",
            Self::Custom => DEFAULT_OUTPUT_DIR,
        }
    }

    /// Returns the user-scoped (global) output directory for this target.
    /// The path is relative to the user's home directory.
    pub fn user_dir(&self) -> &'static str {
        match self {
            Self::GithubCopilot => ".copilot/skills",
            Self::ClaudeCode => ".claude/skills",
            Self::Cursor => ".cursor/skills",
            Self::Antigravity => ".gemini/skills",
            Self::OpenAICodex => ".codex/skills",
            Self::OpenCode => ".config/opencode/skills",
            Self::Custom => ".agent/skills",
        }
    }

    /// Returns all supported target names for CLI help.
    pub fn all_names() -> &'static [&'static str] {
        &[
            "github-copilot",
            "claude-code",
            "cursor",
            "antigravity",
            "openai-codex",
            "opencode",
            "custom",
        ]
    }
}

impl std::fmt::Display for SkillsTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GithubCopilot => write!(f, "github-copilot"),
            Self::ClaudeCode => write!(f, "claude-code"),
            Self::Cursor => write!(f, "cursor"),
            Self::Antigravity => write!(f, "antigravity"),
            Self::OpenAICodex => write!(f, "openai-codex"),
            Self::OpenCode => write!(f, "opencode"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for SkillsTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github-copilot" | "copilot" => Ok(Self::GithubCopilot),
            "claude-code" | "claude" => Ok(Self::ClaudeCode),
            "cursor" => Ok(Self::Cursor),
            "antigravity" | "gemini" => Ok(Self::Antigravity),
            "openai-codex" | "codex" | "openai" => Ok(Self::OpenAICodex),
            "opencode" | "open-code" => Ok(Self::OpenCode),
            "custom" => Ok(Self::Custom),
            _ => Err(format!(
                "Unknown target '{}'. Valid targets: {}",
                s,
                SkillsTarget::all_names().join(", ")
            )),
        }
    }
}

/// Scope for skills installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillsScope {
    /// Project-level skills (stored in project directory)
    #[default]
    Project,
    /// User-level skills (stored in user home directory)
    User,
}

impl std::fmt::Display for SkillsScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Project => write!(f, "project"),
            Self::User => write!(f, "user"),
        }
    }
}

/// Root configuration structure.
///
/// Maps to the `skills.yaml` file format:
/// ```yaml
/// output: .agent/skills
/// flat: true
/// user_agent: "AgentSkillsBot/1.0"
/// delay_ms: 100
/// max_depth: 25
/// rules:
///   - url: "https://docs.flutter.dev/*"
///     action: "allow"
///   - url: "*/api/*"
///     action: "ignore"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Output directory for generated skills.
    #[serde(default = "default_output")]
    pub output: PathBuf,

    /// If true, creates a flat directory structure (no nested subdirectories).
    #[serde(default)]
    pub flat: bool,

    /// Custom User-Agent string for HTTP requests.
    #[serde(default)]
    pub user_agent: Option<String>,

    /// Delay between requests in milliseconds (polite crawling).
    #[serde(default = "default_delay")]
    pub delay_ms: u64,

    /// Maximum crawl depth.
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,

    /// Request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub request_timeout_secs: u64,

    /// Whether to respect robots.txt.
    #[serde(default = "default_true")]
    pub respect_robots_txt: bool,

    /// Allow subdomains when crawling.
    #[serde(default)]
    pub subdomains: bool,

    /// URL filtering rules.
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// CSS selectors for elements to remove from content.
    #[serde(default = "default_remove_selectors")]
    pub remove_selectors: Vec<String>,

    /// Concurrency limit for parallel page processing.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// Target IDE/agent for skills generation.
    /// When set to a specific target, the output path is determined automatically.
    #[serde(default)]
    pub target: SkillsTarget,

    /// Scope for skills installation (project-level or user-level).
    #[serde(default)]
    pub scope: SkillsScope,
}

fn default_output() -> PathBuf {
    PathBuf::from(DEFAULT_OUTPUT_DIR)
}

fn default_delay() -> u64 {
    DEFAULT_DELAY_MS
}

fn default_max_depth() -> usize {
    DEFAULT_MAX_DEPTH
}

fn default_timeout() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_SECS
}

fn default_true() -> bool {
    true
}

fn default_concurrency() -> usize {
    4
}

/// Default CSS selectors for elements that should be removed from content.
/// These typically contain navigation, ads, or other non-content elements.
fn default_remove_selectors() -> Vec<String> {
    vec![
        "nav".to_string(),
        "footer".to_string(),
        "header".to_string(),
        "script".to_string(),
        "style".to_string(),
        "noscript".to_string(),
        "iframe".to_string(),
        ".toc".to_string(),
        ".table-of-contents".to_string(),
        ".sidebar".to_string(),
        ".navigation".to_string(),
        ".nav".to_string(),
        ".menu".to_string(),
        ".breadcrumb".to_string(),
        ".breadcrumbs".to_string(),
        ".ads".to_string(),
        ".advertisement".to_string(),
        ".cookie-banner".to_string(),
        ".cookie-consent".to_string(),
        "[role='navigation']".to_string(),
        "[role='banner']".to_string(),
        "[role='contentinfo']".to_string(),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output: default_output(),
            flat: false,
            user_agent: None,
            delay_ms: default_delay(),
            max_depth: default_max_depth(),
            request_timeout_secs: default_timeout(),
            respect_robots_txt: true,
            subdomains: false,
            rules: Vec::new(),
            remove_selectors: default_remove_selectors(),
            concurrency: default_concurrency(),
            target: SkillsTarget::default(),
            scope: SkillsScope::default(),
        }
    }
}

impl Config {
    /// Loads configuration from a YAML file.
    ///
    /// # Arguments
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    /// The parsed configuration or an error.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs_err::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(config)
    }

    /// Loads configuration from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("Failed to parse YAML configuration")
    }

    /// Builds a UrlFilter from the configured rules.
    pub fn build_url_filter(&self) -> Result<UrlFilter> {
        UrlFilter::new(&self.rules)
    }

    /// Checks if a URL should be crawled based on the configured rules.
    ///
    /// Rules are evaluated using globset. Ignore rules take precedence,
    /// then allow rules are checked. If allow rules exist, non-matching URLs are ignored.
    pub fn should_crawl(&self, url: &str) -> bool {
        // Build filter on demand (for simple usage)
        match self.build_url_filter() {
            Ok(filter) => filter.should_crawl(url),
            Err(_) => {
                // Fallback to simple matching if filter build fails
                for rule in &self.rules {
                    if rule.matches(url) {
                        return matches!(rule.action, Action::Allow);
                    }
                }
                true
            }
        }
    }

    /// Returns URLs that should be blacklisted (for spider configuration).
    /// These are converted to regex patterns for spider's blacklist_url.
    pub fn get_blacklist_patterns(&self) -> Vec<String> {
        self.rules
            .iter()
            .filter(|r| matches!(r.action, Action::Ignore))
            .map(|r| r.to_regex_pattern())
            .collect()
    }

    /// Returns URLs that should be whitelisted (for spider configuration).
    /// These are converted to regex patterns for spider's whitelist_url.
    pub fn get_whitelist_regex_patterns(&self) -> Vec<String> {
        self.rules
            .iter()
            .filter(|r| matches!(r.action, Action::Allow))
            .map(|r| r.to_regex_pattern())
            .collect()
    }

    /// Returns raw whitelist patterns (glob format).
    pub fn get_whitelist_patterns(&self) -> Vec<String> {
        self.rules
            .iter()
            .filter(|r| matches!(r.action, Action::Allow))
            .map(|r| r.url.clone())
            .collect()
    }

    /// Checks if there are any allow rules configured.
    pub fn has_allow_rules(&self) -> bool {
        self.rules.iter().any(|r| matches!(r.action, Action::Allow))
    }

    /// Resolves the output path based on the target and scope.
    ///
    /// - For `SkillsTarget::Custom`, returns the `output` field as-is.
    /// - For other targets, returns the appropriate project or user directory.
    /// - For user scope, expands `~` to the user's home directory.
    pub fn resolve_output_path(&self) -> PathBuf {
        match self.target {
            SkillsTarget::Custom => self.output.clone(),
            _ => match self.scope {
                SkillsScope::Project => PathBuf::from(self.target.project_dir()),
                SkillsScope::User => {
                    if let Some(home) = dirs_home() {
                        home.join(self.target.user_dir())
                    } else {
                        // Fallback to project directory if home not found
                        PathBuf::from(self.target.project_dir())
                    }
                }
            },
        }
    }
}

/// Returns the user's home directory.
fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

/// A URL filtering rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// URL pattern to match. Supports glob-like patterns:
    /// - `*` matches any sequence of characters
    /// - `?` matches any single character
    pub url: String,

    /// Action to take when the URL matches.
    pub action: Action,

    /// Optional: Only apply this rule to specific content types.
    #[serde(default)]
    pub content_type: Option<String>,
}

impl Rule {
    /// Checks if this rule matches the given URL using globset.
    pub fn matches(&self, url: &str) -> bool {
        match Glob::new(&self.url) {
            Ok(glob) => glob.compile_matcher().is_match(url),
            Err(_) => {
                // If glob compilation fails, fall back to simple contains check
                url.contains(&self.url.replace('*', ""))
            }
        }
    }

    /// Converts the glob pattern to a regex pattern for spider's blacklist.
    pub fn to_regex_pattern(&self) -> String {
        glob_to_regex(&self.url)
    }
}

/// Action to take for matched URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Allow crawling this URL.
    Allow,
    /// Ignore (skip) this URL.
    Ignore,
}

/// Converts a glob-like pattern to a regex pattern.
fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::with_capacity(glob.len() * 2);
    regex.push('^');

    for c in glob.chars() {
        match c {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex.push('\\');
                regex.push(c);
            }
            _ => regex.push(c),
        }
    }

    regex.push('$');
    regex
}

/// URL filter using compiled GlobSet for efficient matching.
///
/// This provides O(n) matching against multiple patterns simultaneously.
#[derive(Debug)]
pub struct UrlFilter {
    /// GlobSet for "allow" patterns.
    allow_set: GlobSet,
    /// GlobSet for "ignore" patterns.
    ignore_set: GlobSet,
    /// Whether we have any allow rules (if so, non-matching URLs are ignored).
    has_allow_rules: bool,
}

impl UrlFilter {
    /// Creates a new URL filter from a list of rules.
    pub fn new(rules: &[Rule]) -> Result<Self> {
        let mut allow_builder = GlobSetBuilder::new();
        let mut ignore_builder = GlobSetBuilder::new();
        let mut has_allow_rules = false;

        for rule in rules {
            let glob = Glob::new(&rule.url)
                .with_context(|| format!("Invalid glob pattern: {}", rule.url))?;

            match rule.action {
                Action::Allow => {
                    allow_builder.add(glob);
                    has_allow_rules = true;
                }
                Action::Ignore => {
                    ignore_builder.add(glob);
                }
            }
        }

        let allow_set = allow_builder
            .build()
            .context("Failed to build allow GlobSet")?;
        let ignore_set = ignore_builder
            .build()
            .context("Failed to build ignore GlobSet")?;

        Ok(Self {
            allow_set,
            ignore_set,
            has_allow_rules,
        })
    }

    /// Checks if a URL should be crawled.
    ///
    /// Logic (ignore rules take precedence over allow rules):
    /// 1. If URL matches any "ignore" pattern, return false
    /// 2. If URL matches any "allow" pattern, return true
    /// 3. If we have "allow" rules but URL doesn't match, return false
    /// 4. If we have no "allow" rules and not ignored, return true (default allow)
    pub fn should_crawl(&self, url: &str) -> bool {
        // First check ignore patterns - these take precedence
        if self.ignore_set.is_match(url) {
            return false;
        }

        // Then check allow patterns
        if self.allow_set.is_match(url) {
            return true;
        }

        // If we have allow rules but URL didn't match any, reject it
        if self.has_allow_rules {
            return false;
        }

        // No allow rules and not ignored = allowed
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.output, PathBuf::from(".agent/skills"));
        assert!(!config.flat);
        assert!(config.respect_robots_txt);
        assert_eq!(config.delay_ms, 100);
    }

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
output: ./output
flat: true
delay_ms: 200
rules:
  - url: "https://docs.flutter.dev/*"
    action: allow
  - url: "*/api/internal/*"
    action: ignore
"#;

        let config = Config::from_yaml(yaml).unwrap();
        assert_eq!(config.output, PathBuf::from("./output"));
        assert!(config.flat);
        assert_eq!(config.delay_ms, 200);
        assert_eq!(config.rules.len(), 2);
    }

    #[test]
    fn test_rule_matching() {
        let rule = Rule {
            url: "https://docs.flutter.dev/*".to_string(),
            action: Action::Allow,
            content_type: None,
        };

        assert!(rule.matches("https://docs.flutter.dev/get-started"));
        assert!(rule.matches("https://docs.flutter.dev/api/widgets"));
        assert!(!rule.matches("https://flutter.dev/docs"));
    }

    #[test]
    fn test_should_crawl() {
        let config = Config::from_yaml(
            r#"
rules:
  - url: "*/internal/*"
    action: ignore
  - url: "*/docs/*"
    action: allow
"#,
        )
        .unwrap();

        assert!(config.should_crawl("https://example.com/docs/api"));
        assert!(!config.should_crawl("https://example.com/internal/admin"));
        // With allow rules present, non-matching URLs are rejected
        assert!(!config.should_crawl("https://example.com/public"));
    }

    #[test]
    fn test_ignore_takes_precedence_over_allow() {
        // Test that ignore rules take precedence when a URL matches both
        // an allow pattern and an ignore pattern
        let config = Config::from_yaml(
            r#"
rules:
  - url: "https://pub.dev/packages/camera"
    action: allow
  - url: "https://pub.dev/packages/camera/**"
    action: allow
  - url: "*/versions/*"
    action: ignore
  - url: "*/versions"
    action: ignore
"#,
        )
        .unwrap();

        // URL under allowed path but NOT matching ignore pattern should be allowed
        assert!(config.should_crawl("https://pub.dev/packages/camera"));
        assert!(config.should_crawl("https://pub.dev/packages/camera/example"));
        assert!(config.should_crawl("https://pub.dev/packages/camera/changelog"));

        // URL matching BOTH allow and ignore patterns should be IGNORED
        // (ignore takes precedence)
        assert!(!config.should_crawl("https://pub.dev/packages/camera/versions/0.10.6"));
        assert!(!config.should_crawl("https://pub.dev/packages/camera/versions/0.9.4"));
        assert!(!config.should_crawl("https://pub.dev/packages/camera/versions"));
    }

    #[test]
    fn test_should_crawl_no_allow_rules() {
        // When only ignore rules exist, non-matching URLs are allowed
        let config = Config::from_yaml(
            r#"
rules:
  - url: "*/internal/*"
    action: ignore
"#,
        )
        .unwrap();

        assert!(config.should_crawl("https://example.com/docs/api"));
        assert!(!config.should_crawl("https://example.com/internal/admin"));
        assert!(config.should_crawl("https://example.com/public")); // No allow rules, default allow
    }

    #[test]
    fn test_glob_to_regex() {
        assert_eq!(glob_to_regex("*.txt"), "^.*\\.txt$");
        assert_eq!(glob_to_regex("hello?world"), "^hello.world$");
        assert_eq!(glob_to_regex("test[1]"), "^test\\[1\\]$");
    }

    #[test]
    fn test_url_pattern_matching() {
        // Test that URL patterns match correctly
        let config = Config::from_yaml(
            r#"
rules:
  - url: "https://docs.flutter.dev/ui/*"
    action: allow
"#,
        )
        .unwrap();

        // The base URL with trailing slash should match (glob * matches zero or more)
        assert!(config.should_crawl("https://docs.flutter.dev/ui/"));
        assert!(config.should_crawl("https://docs.flutter.dev/ui/widgets"));
        assert!(config.should_crawl("https://docs.flutter.dev/ui/widgets/buttons"));
        assert!(!config.should_crawl("https://docs.flutter.dev/cookbook/"));
        assert!(!config.should_crawl("https://docs.flutter.dev/"));
    }

    #[test]
    fn test_skills_target_default() {
        let config = Config::default();
        assert_eq!(config.target, SkillsTarget::Custom);
        assert_eq!(config.scope, SkillsScope::Project);
    }

    #[test]
    fn test_skills_target_project_dirs() {
        assert_eq!(SkillsTarget::GithubCopilot.project_dir(), ".github/skills");
        assert_eq!(SkillsTarget::ClaudeCode.project_dir(), ".claude/skills");
        assert_eq!(SkillsTarget::Cursor.project_dir(), ".cursor/skills");
        assert_eq!(SkillsTarget::Antigravity.project_dir(), ".gemini/skills");
        assert_eq!(SkillsTarget::OpenAICodex.project_dir(), ".codex/skills");
        assert_eq!(SkillsTarget::OpenCode.project_dir(), ".opencode/skills");
        assert_eq!(SkillsTarget::Custom.project_dir(), ".agent/skills");
    }

    #[test]
    fn test_skills_target_user_dirs() {
        assert_eq!(SkillsTarget::GithubCopilot.user_dir(), ".copilot/skills");
        assert_eq!(SkillsTarget::ClaudeCode.user_dir(), ".claude/skills");
        assert_eq!(SkillsTarget::Cursor.user_dir(), ".cursor/skills");
        assert_eq!(SkillsTarget::Antigravity.user_dir(), ".gemini/skills");
        assert_eq!(SkillsTarget::OpenAICodex.user_dir(), ".codex/skills");
        assert_eq!(SkillsTarget::OpenCode.user_dir(), ".config/opencode/skills");
        assert_eq!(SkillsTarget::Custom.user_dir(), ".agent/skills");
    }

    #[test]
    fn test_skills_target_from_str() {
        assert_eq!(
            "cursor".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::Cursor
        );
        assert_eq!(
            "claude-code".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::ClaudeCode
        );
        assert_eq!(
            "claude".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::ClaudeCode
        );
        assert_eq!(
            "github-copilot".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::GithubCopilot
        );
        assert_eq!(
            "copilot".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::GithubCopilot
        );
        assert_eq!(
            "antigravity".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::Antigravity
        );
        assert_eq!(
            "gemini".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::Antigravity
        );
        assert_eq!(
            "openai-codex".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::OpenAICodex
        );
        assert_eq!(
            "codex".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::OpenAICodex
        );
        assert_eq!(
            "opencode".parse::<SkillsTarget>().unwrap(),
            SkillsTarget::OpenCode
        );
        assert!("invalid".parse::<SkillsTarget>().is_err());
    }

    #[test]
    fn test_config_yaml_with_target() {
        let yaml = r#"
target: cursor
scope: user
output: ./custom-output
"#;
        let config = Config::from_yaml(yaml).unwrap();
        assert_eq!(config.target, SkillsTarget::Cursor);
        assert_eq!(config.scope, SkillsScope::User);
    }

    #[test]
    fn test_resolve_output_path_custom() {
        let config = Config {
            target: SkillsTarget::Custom,
            output: PathBuf::from("./my-skills"),
            ..Default::default()
        };
        assert_eq!(config.resolve_output_path(), PathBuf::from("./my-skills"));
    }

    #[test]
    fn test_resolve_output_path_project() {
        let config = Config {
            target: SkillsTarget::Cursor,
            scope: SkillsScope::Project,
            ..Default::default()
        };
        assert_eq!(
            config.resolve_output_path(),
            PathBuf::from(".cursor/skills")
        );
    }
}
