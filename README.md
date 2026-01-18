# Agent Skills Generator

A production-grade CLI for crawling documentation sites and generating agent-ready skills using the Reference Pattern. Each crawled page becomes a single `SKILL.md` with metadata plus full Markdown content for fast retrieval and low token overhead.

## Features

- Polite, configurable crawling (delay, depth, robots.txt)
- Rule-based URL filtering with glob patterns (allow/ignore)
- HTML cleaning to remove navs/ads/sidebars
- HTML → Markdown conversion
- Consolidated output: one `SKILL.md` per page
- Single-URL processing for quick testing

## Installation

### From source (local)

```bash
cargo install --path .
```

### Build a release binary

```bash
cargo build --release
```

Run with:

```bash
./target/release/agent-skills-generator --help
```

## Quick start

```bash
# Create a default config
agent-skills-generator init

# Crawl a documentation site
agent-skills-generator crawl https://docs.example.com

# Validate config
agent-skills-generator validate --show

# Clean generated skills
agent-skills-generator clean
```

## CLI overview

- `crawl <URL...>`: Crawl one or more starting URLs.
- `single <URL>`: Process a single page without crawling.
- `validate`: Validate and optionally print the parsed configuration.
- `clean`: Remove generated skill directories.
- `init`: Create a default `skills.yaml`.

Use `-v` / `-vv` for more logs or `-q` for quiet mode.

## URL patterns

You can pass glob patterns directly to `crawl` (e.g., `*` or `?`). The tool auto-generates allow/ignore rules to constrain the crawl to matching paths.

Example:

```bash
agent-skills-generator crawl "https://docs.example.com/api/*"
```

## Configuration

The default config file is `skills.yaml`. Example:

```yaml
output: .agent/skills
flat: false
user_agent: "AgentSkillsBot/1.0"
delay_ms: 100
max_depth: 25
request_timeout_secs: 30
respect_robots_txt: true
subdomains: false
concurrency: 4

rules:
  - url: "*/docs/*"
    action: allow
  - url: "*/login*"
    action: ignore

remove_selectors:
  - ".custom-sidebar"
  - "#ad-container"
```

### Rule behavior

- Rules are evaluated in order.
- If any `allow` rules exist, non-matching URLs are skipped.
- `ignore` rules always skip matching URLs.
- Patterns use glob syntax (`*`, `?`).

## Output structure

Each crawled page produces a directory named from the URL path, sanitized to kebab-case (max 64 chars).

```text
.agent/skills/
  docs-getting-started/
    SKILL.md
  api-reference/
    SKILL.md
```

### SKILL.md format

```markdown
---
name: docs-getting-started
description: A concise description from metadata or first paragraph.
metadata:
  url: https://docs.example.com/getting-started
---

# Page Title

...converted markdown content...
```

## Development

```bash
cargo test
```

## License

MIT
