# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run a single test
cargo test test_name

# Lint with clippy (CI uses -D warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Format check
cargo fmt --check

# Format code
cargo fmt

# Run with debug logging
RUST_LOG=debug cargo run -- crawl https://example.com
```

## Architecture Overview

This is a Rust CLI tool that crawls documentation websites and generates SKILL.md files optimized for AI agents. Built with async Tokio runtime and the spider crate for web crawling.

### Module Structure

```
src/
├── main.rs      # Entry point, command dispatch, config loading
├── cli.rs       # CLI argument parsing with clap (Commands enum)
├── config.rs    # YAML config loading, URL filtering rules (GlobSet-based)
├── crawler.rs   # Async web crawler using spider crate with page subscription
├── processor.rs # HTML cleaning, markdown conversion, SKILL.md generation
└── utils.rs     # String sanitization, URL path extraction, truncation
```

### Data Flow

1. **CLI** parses args → loads `skills.yaml` config
2. **Crawler** subscribes to spider's page events with concurrency control (Semaphore)
3. **Processor** receives each page:
   - Cleans HTML (removes nav, scripts, styles, ads via regex patterns)
   - Extracts metadata (title, description, skill name from URL path)
   - Converts HTML to Markdown using htmd crate
   - Post-processes markdown to remove icon names and noise
4. **Output**: `<output_dir>/<skill-name>/SKILL.md` with YAML frontmatter

### Key Types

- `Config` - Root configuration from skills.yaml, includes `UrlFilter` for allow/ignore rules
- `SkillsTarget` - Enum for IDE/agent targets (GithubCopilot, ClaudeCode, Cursor, Antigravity, OpenAICodex, OpenCode, Custom)
- `SkillsScope` - Enum for Project or User level installation
- `Crawler` - Owns spider Website, Processor, and CrawlStats
- `Processor` - Stateless HTML→Markdown transformer
- `ProcessedPage` - Contains metadata, cleaned_html, markdown_content, skill_md

### URL Filtering

Rules in config use glob patterns (via globset crate). Allow rules take precedence over ignore rules. When allow rules exist, non-matching URLs are rejected.

### Configuration

Default config file: `skills.yaml` (created via `agent-skills-generator init`)
Default output: `.agent/skills/` (can be changed via `--target` for IDE-specific paths)

Key CLI options:
- `--target <target>` - Set IDE target (cursor, claude-code, github-copilot, antigravity, openai-codex, opencode)
- `--user` - Install to user-level directory instead of project

## Rust Edition

Uses Rust 2024 edition with `let-else` and `if-let` chains (requires Rust 1.75+).
