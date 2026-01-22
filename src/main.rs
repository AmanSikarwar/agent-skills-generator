//! # Agent Skills Generator
//!
//! A production-grade CLI tool for crawling websites and generating agent skills.
//!
//! ## Consolidated Output
//!
//! All content is written to a single `SKILL.md` file per page containing:
//! - YAML frontmatter with metadata (name, description, url)
//! - Page title
//! - Full converted markdown content
//!
//! ## Usage
//!
//! ```bash
//! # Initialize a new configuration file
//! agent-skills-generator init
//!
//! # Crawl a website
//! agent-skills-generator crawl https://docs.example.com
//!
//! # Clean generated skills
//! agent-skills-generator clean
//!
//! # Validate configuration
//! agent-skills-generator validate
//! ```
//!
//! ## Directory Structure
//!
//! Generated skills follow this structure:
//!
//! ```text
//! .agent/skills/
//!   docs-getting-started/
//!     SKILL.md           # Contains ALL content
//!   docs-api-reference/
//!     SKILL.md           # Contains ALL content
//! ```

pub mod cli;
pub mod config;
pub mod crawler;
pub mod processor;
pub mod utils;

use anyhow::{Context, Result};
use cli::{Cli, Commands, DEFAULT_CONFIG};
use config::{Action, Config, Rule, SkillsScope};
use crawler::{Crawler, clean_output_dir};
use processor::Processor;
use std::io::{self, Write};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use utils::{extract_domain_with_protocol, parse_url_pattern};

/// Main entry point for the CLI application.
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse_args();

    // Initialize logging
    init_logging(&cli);

    // Execute the requested command
    match &cli.command {
        Commands::Crawl(args) => run_crawl(&cli, args).await,
        Commands::Clean(args) => run_clean(&cli, args).await,
        Commands::Validate(args) => run_validate(&cli, args),
        Commands::Single(args) => run_single(&cli, args).await,
        Commands::Init(args) => run_init(args),
    }
}

/// Initialize the tracing subscriber for logging.
fn init_logging(cli: &Cli) {
    let level = cli.log_level();

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.to_string()));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .init();
}

/// Run the crawl command.
async fn run_crawl(cli: &Cli, args: &cli::CrawlArgs) -> Result<()> {
    // Load configuration
    let mut config = load_config(&cli.config)?;

    // Apply command-line overrides
    apply_cli_overrides(&mut config, cli);

    if let Some(delay) = args.delay {
        config.delay_ms = delay;
    }
    if let Some(depth) = args.depth {
        config.max_depth = depth;
    }
    if args.subdomains {
        config.subdomains = true;
    }

    // Determine output directory (CLI --output overrides resolve_output_path)
    let output_dir = if let Some(ref output) = cli.output {
        output.clone()
    } else {
        config.resolve_output_path()
    };

    info!("Output directory: {}", output_dir.display());

    if args.dry_run {
        info!("Dry run mode - no files will be written");
    }

    // Process each URL - parse patterns and crawl
    for url_input in &args.urls {
        let (base_url, pattern) = parse_url_pattern(url_input);

        info!("Crawling: {} (base: {})", url_input, base_url);

        // Auto-generate rules to scope crawling to the initial URL path
        let crawl_config = if let Some(ref url_pattern) = pattern {
            let mut crawl_config = config.clone();

            // Get the domain to create a catch-all ignore rule
            if let Some(domain) = extract_domain_with_protocol(&base_url) {
                info!(
                    "URL pattern detected. Allowing: {}, ignoring other paths on {}",
                    url_pattern, domain
                );

                // Insert rules at the beginning (they take precedence)
                // First: allow the exact base URL (for the starting page)
                crawl_config.rules.insert(
                    0,
                    Rule {
                        url: base_url.clone(),
                        action: Action::Allow,
                        content_type: None,
                    },
                );

                // Second: allow the pattern (use ** for nested paths)
                // Convert trailing /* to /** for recursive matching
                let recursive_pattern = if url_pattern.ends_with("/*") {
                    format!("{}**", &url_pattern[..url_pattern.len() - 1])
                } else {
                    url_pattern.clone()
                };
                crawl_config.rules.insert(
                    1,
                    Rule {
                        url: recursive_pattern,
                        action: Action::Allow,
                        content_type: None,
                    },
                );

                // Third: ignore everything else on this domain
                crawl_config.rules.insert(
                    2,
                    Rule {
                        url: format!("{}/**", domain),
                        action: Action::Ignore,
                        content_type: None,
                    },
                );
            }

            crawl_config
        } else {
            // No explicit pattern - auto-scope to the initial URL prefix
            let mut crawl_config = config.clone();

            // Only add scoping rules if we can extract the domain (valid URL)
            if extract_domain_with_protocol(&base_url).is_some() {
                // Normalize the base URL (ensure it ends with / for directory-style URLs)
                let normalized_base = if base_url.ends_with('/') {
                    base_url.clone()
                } else {
                    format!("{}/", base_url)
                };

                info!("Auto-scoping crawl to URL prefix: {}**", normalized_base);

                // Allow the exact base URL
                crawl_config.rules.insert(
                    0,
                    Rule {
                        url: base_url.clone(),
                        action: Action::Allow,
                        content_type: None,
                    },
                );

                // Allow all URLs under the base URL path
                crawl_config.rules.insert(
                    1,
                    Rule {
                        url: format!("{}**", normalized_base),
                        action: Action::Allow,
                        content_type: None,
                    },
                );

                // Note: We don't add a domain-scope ignore rule here because:
                // 1. The whitelist (allow rules) already restricts spider to matching URLs
                // 2. should_crawl() returns false for URLs not matching any allow pattern
                // 3. Adding a domain-scope ignore would conflict with user-defined ignore rules
            }

            crawl_config
        };

        if args.dry_run {
            info!("Would crawl: {}", base_url);
            info!("Active rules:");
            for (i, rule) in crawl_config.rules.iter().enumerate() {
                info!("  {}. {} -> {:?}", i + 1, rule.url, rule.action);
            }
            continue;
        }

        // Create crawler with the (possibly modified) config
        let crawler = Crawler::new(crawl_config, output_dir.clone())?;

        match crawler.crawl(&base_url).await {
            Ok(stats) => {
                info!("{}", stats.summary());
            }
            Err(e) => {
                error!("Failed to crawl {}: {:?}", base_url, e);
            }
        }
    }

    Ok(())
}

/// Run the clean command.
async fn run_clean(cli: &Cli, args: &cli::CleanArgs) -> Result<()> {
    // Load configuration to get output directory
    let mut config = load_config_or_default(&cli.config);
    apply_cli_overrides(&mut config, cli);

    let output_dir = if let Some(ref output) = cli.output {
        output.clone()
    } else {
        config.resolve_output_path()
    };

    if !output_dir.exists() {
        info!("Output directory does not exist: {}", output_dir.display());
        return Ok(());
    }

    // Confirm unless --force is specified
    if !args.force {
        print!(
            "Are you sure you want to clean all skills in {}? [y/N] ",
            output_dir.display()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            info!("Aborted.");
            return Ok(());
        }
    }

    // Clean the directory
    let count = clean_output_dir(&output_dir).await?;
    info!("Removed {} skill directories", count);

    Ok(())
}

/// Run the validate command.
fn run_validate(cli: &Cli, args: &cli::ValidateArgs) -> Result<()> {
    let mut config = load_config(&cli.config)?;
    apply_cli_overrides(&mut config, cli);

    info!("Configuration is valid!");

    if args.show {
        println!("\n--- Parsed Configuration ---");
        println!("Target: {}", config.target);
        println!("Scope: {}", config.scope);
        println!("Output: {}", config.resolve_output_path().display());
        println!("Flat: {}", config.flat);
        println!("Delay: {}ms", config.delay_ms);
        println!("Max Depth: {}", config.max_depth);
        println!("Respect robots.txt: {}", config.respect_robots_txt);
        println!("Subdomains: {}", config.subdomains);
        println!("Concurrency: {}", config.concurrency);
        println!("Rules: {} defined", config.rules.len());

        for (i, rule) in config.rules.iter().enumerate() {
            println!("  {}. {} -> {:?}", i + 1, rule.url, rule.action);
        }

        println!(
            "Remove selectors: {} defined",
            config.remove_selectors.len()
        );
    }

    Ok(())
}

/// Run the single command - process a single URL.
async fn run_single(cli: &Cli, args: &cli::SingleArgs) -> Result<()> {
    let mut config = load_config_or_default(&cli.config);
    apply_cli_overrides(&mut config, cli);

    let output_dir = if let Some(ref output) = cli.output {
        output.clone()
    } else {
        config.resolve_output_path()
    };

    info!("Processing single URL: {}", args.url);

    // Fetch the page
    let client = reqwest::Client::builder()
        .user_agent("AgentSkillsGenerator/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client
        .get(&args.url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch URL: {}", args.url))?;

    let html = response
        .text()
        .await
        .with_context(|| format!("Failed to read response body from: {}", args.url))?;

    // Process the page
    let processor = Processor::new(&config)?;
    let processed = processor.process(&args.url, &html)?;

    if args.stdout {
        // Output to stdout
        println!("--- SKILL.md ---");
        println!("{}", processed.skill_md);
        println!("\n--- content.md ---");
        println!("{}", processed.markdown_content);
    } else {
        // Write to disk
        fs_err::tokio::create_dir_all(&output_dir).await?;
        let skill_dir = processor.write_to_disk(&processed, &output_dir).await?;
        info!("Written to: {}", skill_dir.display());
    }

    Ok(())
}

/// Run the init command - create a new configuration file.
fn run_init(args: &cli::InitArgs) -> Result<()> {
    if args.path.exists() && !args.force {
        anyhow::bail!(
            "Configuration file already exists: {}. Use --force to overwrite.",
            args.path.display()
        );
    }

    // If --no-interactive, use default config
    if args.no_interactive {
        fs_err::write(&args.path, DEFAULT_CONFIG).with_context(|| {
            format!(
                "Failed to write configuration file: {}",
                args.path.display()
            )
        })?;

        info!("Created configuration file: {}", args.path.display());
        info!("Edit this file to customize crawling behavior, then run:");
        info!("  agent-skills-generator crawl <URL>");

        return Ok(());
    }

    // Interactive mode
    let config_content = run_interactive_init()?;

    fs_err::write(&args.path, &config_content).with_context(|| {
        format!(
            "Failed to write configuration file: {}",
            args.path.display()
        )
    })?;

    info!("Created configuration file: {}", args.path.display());
    info!("Run the following command to start crawling:");
    info!("  agent-skills-generator crawl <URL>");

    Ok(())
}

/// Run interactive initialization prompts and return the generated YAML config.
fn run_interactive_init() -> Result<String> {
    use config::{SkillsScope, SkillsTarget};
    use inquire::{Select, Text};

    // Target IDE selection
    let target_options = [
        ("Custom (specify output path)", SkillsTarget::Custom),
        ("GitHub Copilot", SkillsTarget::GithubCopilot),
        ("Claude Code", SkillsTarget::ClaudeCode),
        ("Cursor", SkillsTarget::Cursor),
        ("Antigravity (Gemini)", SkillsTarget::Antigravity),
        ("OpenAI Codex", SkillsTarget::OpenAICodex),
        ("OpenCode", SkillsTarget::OpenCode),
    ];

    let target_names: Vec<&str> = target_options.iter().map(|(name, _)| *name).collect();
    let target_idx = Select::new("Select target IDE/agent:", target_names)
        .with_help_message("Choose where your skills will be installed")
        .prompt()
        .map_err(|e| anyhow::anyhow!("Failed to get target selection: {}", e))?;

    let target = target_options
        .iter()
        .find(|(name, _)| *name == target_idx)
        .map(|(_, t)| *t)
        .unwrap_or(SkillsTarget::Custom);

    // Scope selection
    let scope_options = [
        (
            "Project (install to current directory)",
            SkillsScope::Project,
        ),
        ("User (install to home directory)", SkillsScope::User),
    ];

    let scope_names: Vec<&str> = scope_options.iter().map(|(name, _)| *name).collect();
    let scope_idx = Select::new("Install skills at project or user level?", scope_names)
        .with_help_message("Project-level is recommended for team collaboration")
        .prompt()
        .map_err(|e| anyhow::anyhow!("Failed to get scope selection: {}", e))?;

    let scope = scope_options
        .iter()
        .find(|(name, _)| *name == scope_idx)
        .map(|(_, s)| *s)
        .unwrap_or(SkillsScope::Project);

    // Output path (only for custom target)
    let output = if matches!(target, SkillsTarget::Custom) {
        Text::new("Output directory:")
            .with_default(".agent/skills")
            .with_help_message("Where to store generated skill files")
            .prompt()
            .map_err(|e| anyhow::anyhow!("Failed to get output path: {}", e))?
    } else {
        ".agent/skills".to_string()
    };

    // Crawl settings
    let delay_ms = Text::new("Request delay in milliseconds:")
        .with_default("100")
        .with_help_message("Delay between requests for polite crawling")
        .prompt()
        .map_err(|e| anyhow::anyhow!("Failed to get delay: {}", e))?
        .parse::<u64>()
        .unwrap_or(100);

    let max_depth = Text::new("Maximum crawl depth:")
        .with_default("25")
        .with_help_message("How deep to follow links from the starting URL")
        .prompt()
        .map_err(|e| anyhow::anyhow!("Failed to get max depth: {}", e))?
        .parse::<usize>()
        .unwrap_or(25);

    let concurrency = Text::new("Concurrency limit:")
        .with_default("4")
        .with_help_message("Number of parallel requests")
        .prompt()
        .map_err(|e| anyhow::anyhow!("Failed to get concurrency: {}", e))?
        .parse::<usize>()
        .unwrap_or(4);

    // Generate YAML configuration
    let config_yaml = format!(
        r##"# Agent Skills Generator Configuration
# See https://github.com/agentskills/agentskills for documentation

# Target IDE/agent for skills generation
# Supported targets: github-copilot, claude-code, cursor, antigravity, openai-codex, opencode, custom
target: {}

# Scope for skills installation
# - project: Install to project directory (e.g., .cursor/skills/)
# - user: Install to user home directory (e.g., ~/.cursor/skills/)
scope: {}

# Output directory for generated skills (only used when target is "custom")
output: {}

# Create flat directory structure (no subdirectories)
flat: false

# Delay between requests in milliseconds (polite crawling)
delay_ms: {}

# Maximum crawl depth
max_depth: {}

# Request timeout in seconds
request_timeout_secs: 30

# Respect robots.txt
respect_robots_txt: true

# Allow subdomains
subdomains: false

# Concurrency limit for parallel page processing
concurrency: {}

# URL filtering rules (evaluated in order)
rules:
  # Example: Allow only documentation pages
  # - url: "*/docs/*"
  #   action: allow

  # Example: Ignore API internals
  # - url: "*/api/internal/*"
  #   action: ignore

# CSS selectors for elements to remove from content
# These are already included by default, add more if needed:
# remove_selectors:
#   - ".custom-sidebar"
#   - "#ad-container"
"##,
        target, scope, output, delay_ms, max_depth, concurrency
    );

    Ok(config_yaml)
}

/// Load configuration from file.
fn load_config(path: &std::path::Path) -> Result<Config> {
    if !path.exists() {
        anyhow::bail!(
            "Configuration file not found: {}. Run 'agent-skills-generator init' to create one.",
            path.display()
        );
    }

    Config::load(path)
}

/// Load configuration from file, or return default if file doesn't exist.
fn load_config_or_default(path: &std::path::Path) -> Config {
    if path.exists() {
        match Config::load(path) {
            Ok(config) => config,
            Err(e) => {
                warn!("Failed to load config, using defaults: {:?}", e);
                Config::default()
            }
        }
    } else {
        Config::default()
    }
}

/// Apply CLI overrides to configuration.
///
/// This applies the following CLI flags to the configuration:
/// - `--target`: Sets the target IDE/agent
/// - `--user`: Sets the scope to user-level
fn apply_cli_overrides(config: &mut Config, cli: &Cli) {
    // Apply target override
    if let Some(target) = cli.target {
        config.target = target;
    }

    // Apply user-level scope override
    if cli.user_level {
        config.scope = SkillsScope::User;
    }
}
