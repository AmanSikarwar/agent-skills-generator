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

        // If URL contains a pattern, auto-generate rules
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
            config.clone()
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

    fs_err::write(&args.path, DEFAULT_CONFIG).with_context(|| {
        format!(
            "Failed to write configuration file: {}",
            args.path.display()
        )
    })?;

    info!("Created configuration file: {}", args.path.display());
    info!("Edit this file to customize crawling behavior, then run:");
    info!("  agent-skills-generator crawl <URL>");

    Ok(())
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
