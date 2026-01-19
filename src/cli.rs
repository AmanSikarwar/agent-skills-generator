//! CLI module for the agent-skills-generator.
//!
//! This module defines the command-line interface using the `clap` derive pattern.
//! It supports the following subcommands:
//!
//! - `crawl` - Crawl a website and generate skill files
//! - `clean` - Remove all generated skill files
//! - `validate` - Validate the configuration file

use clap::{Args, Parser, Subcommand};
use std::path::{Path, PathBuf};

/// Agent Skills Generator - A production-grade CLI tool for crawling websites
/// and generating agent skills following the Reference Pattern.
///
/// The Reference Pattern keeps SKILL.md files lightweight (metadata + reference link)
/// to minimize token usage when skills are loaded into LLM context.
#[derive(Parser, Debug)]
#[command(
    name = "agent-skills-generator",
    author = "Agent Skills Contributors",
    version,
    about = "Generate agent skills from web documentation",
    long_about = None,
    propagate_version = true
)]
pub struct Cli {
    /// Path to the configuration file.
    #[arg(
        short,
        long,
        default_value = "skills.yaml",
        global = true,
        env = "SKILLS_CONFIG"
    )]
    pub config: PathBuf,

    /// Output directory for generated skills.
    /// Overrides the value in the config file.
    #[arg(short, long, global = true, env = "SKILLS_OUTPUT")]
    pub output: Option<PathBuf>,

    /// Enable verbose logging.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all output except errors.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Crawl a website and generate skill files.
    ///
    /// This command will:
    /// 1. Parse the configuration file
    /// 2. Crawl the specified URL(s)
    /// 3. Clean HTML and extract content
    /// 4. Generate SKILL.md files with the Reference Pattern
    Crawl(CrawlArgs),

    /// Remove all generated skill files from the output directory.
    ///
    /// Only removes directories that contain a SKILL.md file,
    /// preserving any manually created files.
    Clean(CleanArgs),

    /// Validate the configuration file.
    ///
    /// Checks for:
    /// - Valid YAML syntax
    /// - Correct field types
    /// - Valid regex patterns in rules
    Validate(ValidateArgs),

    /// Process a single URL without crawling.
    ///
    /// Useful for testing or processing individual pages.
    Single(SingleArgs),

    /// Initialize a new configuration file.
    ///
    /// Creates a default skills.yaml file in the current directory.
    Init(InitArgs),
}

/// Arguments for the `crawl` subcommand.
#[derive(Args, Debug)]
pub struct CrawlArgs {
    /// The URL(s) to crawl.
    ///
    /// You can specify multiple URLs to crawl from different starting points.
    #[arg(required = true)]
    pub urls: Vec<String>,

    /// Maximum number of pages to crawl.
    ///
    /// Use this to limit the scope of the crawl for testing.
    #[arg(short, long)]
    pub max_pages: Option<usize>,

    /// Crawl delay in milliseconds.
    /// Overrides the value in the config file.
    #[arg(short, long)]
    pub delay: Option<u64>,

    /// Maximum crawl depth.
    /// Overrides the value in the config file.
    #[arg(long)]
    pub depth: Option<usize>,

    /// Follow subdomains.
    #[arg(long)]
    pub subdomains: bool,

    /// Dry run - don't write any files, just show what would be done.
    #[arg(long)]
    pub dry_run: bool,

    /// Continue from a previous crawl (skip existing skills).
    #[arg(long)]
    pub resume: bool,
}

/// Arguments for the `clean` subcommand.
#[derive(Args, Debug)]
pub struct CleanArgs {
    /// Don't ask for confirmation before cleaning.
    #[arg(short, long)]
    pub force: bool,

    /// Only remove skills matching this pattern.
    #[arg(short, long)]
    pub pattern: Option<String>,
}

/// Arguments for the `validate` subcommand.
#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Show the parsed configuration.
    #[arg(short, long)]
    pub show: bool,
}

/// Arguments for the `single` subcommand.
#[derive(Args, Debug)]
pub struct SingleArgs {
    /// The URL to process.
    #[arg(required = true)]
    pub url: String,

    /// Output to stdout instead of writing files.
    #[arg(long)]
    pub stdout: bool,
}

/// Arguments for the `init` subcommand.
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Overwrite existing configuration file.
    #[arg(short, long)]
    pub force: bool,

    /// Path where to create the configuration file.
    #[arg(short, long, default_value = "skills.yaml")]
    pub path: PathBuf,
}

impl Cli {
    /// Parse command-line arguments.
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Get the effective output directory.
    ///
    /// Prefers command-line argument over config file value.
    pub fn effective_output(&self, config_output: &Path) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| config_output.to_path_buf())
    }

    /// Get the log level based on verbosity flags.
    pub fn log_level(&self) -> tracing::Level {
        if self.quiet {
            tracing::Level::ERROR
        } else {
            match self.verbose {
                0 => tracing::Level::INFO,
                1 => tracing::Level::DEBUG,
                _ => tracing::Level::TRACE,
            }
        }
    }
}

/// Default configuration file content.
pub const DEFAULT_CONFIG: &str = r##"# Agent Skills Generator Configuration
# See https://github.com/agentskills/agentskills for documentation

# Output directory for generated skills
output: .agent/skills

# Create flat directory structure (no subdirectories)
flat: false

# Custom User-Agent string
# user_agent: "MyBot/1.0"

# Delay between requests in milliseconds (polite crawling)
delay_ms: 100

# Maximum crawl depth
max_depth: 25

# Request timeout in seconds
request_timeout_secs: 30

# Respect robots.txt
respect_robots_txt: true

# Allow subdomains
subdomains: false

# Concurrency limit for parallel page processing
concurrency: 4

# URL filtering rules (evaluated in order)
rules:
  # Example: Allow only documentation pages
  # - url: "*/docs/*"
  #   action: allow

  # Example: Ignore API internals
  # - url: "*/api/internal/*"
  #   action: ignore

  # Example: Ignore login/auth pages
  # - url: "*/login*"
  #   action: ignore
  # - url: "*/auth/*"
  #   action: ignore

# CSS selectors for elements to remove from content
# These are already included by default, add more if needed:
# remove_selectors:
#   - ".custom-sidebar"
#   - "#ad-container"
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::parse_from([
            "agent-skills-generator",
            "-v",
            "crawl",
            "https://example.com",
        ]);

        assert_eq!(cli.verbose, 1);
        assert!(matches!(cli.command, Commands::Crawl(_)));

        if let Commands::Crawl(args) = cli.command {
            assert_eq!(args.urls, vec!["https://example.com"]);
        }
    }

    #[test]
    fn test_clean_command() {
        let cli = Cli::parse_from(["agent-skills-generator", "clean", "--force"]);

        if let Commands::Clean(args) = cli.command {
            assert!(args.force);
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_validate_command() {
        let cli = Cli::parse_from(["agent-skills-generator", "validate", "--show"]);

        if let Commands::Validate(args) = cli.command {
            assert!(args.show);
        } else {
            panic!("Expected Validate command");
        }
    }

    #[test]
    fn test_log_level() {
        let cli = Cli::parse_from(["agent-skills-generator", "clean"]);
        assert_eq!(cli.log_level(), tracing::Level::INFO);

        let cli = Cli::parse_from(["agent-skills-generator", "-v", "clean"]);
        assert_eq!(cli.log_level(), tracing::Level::DEBUG);

        let cli = Cli::parse_from(["agent-skills-generator", "-vv", "clean"]);
        assert_eq!(cli.log_level(), tracing::Level::TRACE);

        let cli = Cli::parse_from(["agent-skills-generator", "-q", "clean"]);
        assert_eq!(cli.log_level(), tracing::Level::ERROR);
    }
}
