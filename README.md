# Agent Skills Generator

Production-grade CLI that crawls documentation sites, cleans HTML, converts it to Markdown, and emits one consolidated `SKILL.md` per page. Built around polite crawling, rule-based filtering, and the Reference Pattern so skills stay concise and structured.

## Features

- Crawl entire sites or single pages with rule-based allow/ignore filters (glob patterns compiled via globset).
- HTML noise removal (nav/footers, scripts, ads, buttons, cookie banners, icons) + Markdown cleanup.
- Consolidated output: one `SKILL.md` per page with YAML frontmatter (`name`, `description`, `metadata.url`) plus full Markdown content.
- Configurable politeness: delay, depth, timeout, robots.txt respect, subdomains, concurrency.
- Dry-run, resume-friendly crawling, and a cleaning command that only deletes generated skills.

## Install

- Prereqs: Rust toolchain (2024 edition), `cargo`.
- Build: `cargo build --release`
- Binary will be at `target/release/agent-skills-generator`.

## Quick Start

1. Initialize config: `agent-skills-generator init` (creates `skills.yaml`).
2. Crawl a site: `agent-skills-generator crawl https://docs.example.com`
3. View output: `.agent/skills/<skill-name>/SKILL.md`

## CLI

- `crawl <URL>...` — crawl starting URLs; respects config; options: `--delay <ms>`, `--depth <n>`, `--subdomains`, `--max-pages <n>`, `--dry-run`, `--resume`.
- `single <URL>` — fetch and process one page without crawling. Add `--stdout` to print instead of writing files.
- `clean` — remove generated skill dirs (prompts unless `--force`).
- `validate` — parse and sanity-check config; `--show` prints the parsed values.
- `init` — write a default `skills.yaml` (use `--force` to overwrite).

Global flags: `--config <path>` (default `skills.yaml`), `--output <dir>` override, `-v/-vv` for debug/trace, `-q` quiet.

## Configuration (`skills.yaml`)

```yaml
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
  # - url: "*/docs/*"
  #   action: allow
  # - url: "*/api/internal/*"
  #   action: ignore

# CSS selectors for elements to remove
# remove_selectors:
#   - ".custom-sidebar"
#   - "#ad-container"
```

Key points:

- Rules are glob patterns; `allow` takes precedence, then `ignore`; if any `allow` exists, non-matching URLs are skipped.
- `subdomains: true` permits crossing subdomains; otherwise they are blocked.
- `concurrency` controls concurrent page processing; `delay_ms` is a politeness gap between requests.

## Output Layout

```
.agent/skills/
  docs-getting-started/
    SKILL.md
```

`SKILL.md` structure:

```markdown
---
name: docs-getting-started
description: Short summary capped at 1024 chars
metadata:
  url: https://docs.example.com/getting-started
---

# Page Title

<Full Markdown content>
```

## How It Works

- `crawler` uses `spider` to walk pages with whitelist/blacklist regex derived from rules, respecting robots and configured depth/timeout.
- `processor` strips noisy HTML via regex, converts to Markdown (`htmd`), cleans residual artifacts, and warns for very large pages (>~20k chars).
- Skill names are sanitized to strict kebab-case (`utils::sanitize_skill_name`) and truncated to 64 chars.

## Development

- Run tests: `cargo test`
- Adjust logging with `-v/-vv/-q` or `RUST_LOG` env (tracing + `EnvFilter`).

## Tips

- Start with narrow `rules` (whitelist) to keep crawl focused and fast.
- Use `--dry-run` to inspect which URLs would be processed before writing files.
- Set `concurrency` conservatively for sites with strict rate limits.
- Respect `robots.txt` unless you have permission to override.

## License

MIT
