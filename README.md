<div align="center">

# Agent Skills Generator

**Transform any documentation website into AI-ready skill files**

[![CI](https://github.com/AmanSikarwar/agent-skills-generator/actions/workflows/ci.yml/badge.svg)](https://github.com/AmanSikarwar/agent-skills-generator/actions/workflows/ci.yml)
[![Release](https://github.com/AmanSikarwar/agent-skills-generator/actions/workflows/release.yml/badge.svg)](https://github.com/AmanSikarwar/agent-skills-generator/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

[Installation](#installation) •
[Quick Start](#quick-start) •
[Configuration](#configuration) •
[Examples](#examples) •
[Contributing](#contributing)

</div>

---

## What is Agent Skills Generator?

Agent Skills Generator is a high-performance CLI tool that crawls documentation websites and generates structured **SKILL.md** files optimized for AI agents. These skill files enable LLMs to access up-to-date documentation directly, enhancing their capabilities with domain-specific knowledge.

### Key Features

- **Blazing Fast** — Built in Rust for maximum performance with async crawling
- **Smart Content Extraction** — Automatically removes navigation, ads, and noise
- **Token Optimized** — Clean markdown output minimizes token usage
- **Configurable** — Fine-grained control over crawling behavior with YAML config
- **Polite Crawling** — Respects robots.txt and implements request delays
- **Cross-Platform** — Works on Linux, macOS, and Windows

---

## Installation

### One-Line Install

**Linux / macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/AmanSikarwar/agent-skills-generator/master/install.sh | bash
```

**Windows (PowerShell):**

```powershell
iwr -useb https://raw.githubusercontent.com/AmanSikarwar/agent-skills-generator/master/install.ps1 | iex
```

### Using Cargo

If you have Rust installed:

```bash
cargo install --git https://github.com/AmanSikarwar/agent-skills-generator
```

### From Source

```bash
git clone https://github.com/AmanSikarwar/agent-skills-generator
cd agent-skills-generator
cargo build --release
```

The binary will be at `target/release/agent-skills-generator`.

### Download Binary

Download pre-built binaries from the [Releases](https://github.com/AmanSikarwar/agent-skills-generator/releases) page.

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 | [Download](https://github.com/AmanSikarwar/agent-skills-generator/releases/latest) |
| Linux | ARM64 | [Download](https://github.com/AmanSikarwar/agent-skills-generator/releases/latest) |
| macOS | Intel | [Download](https://github.com/AmanSikarwar/agent-skills-generator/releases/latest) |
| macOS | Apple Silicon | [Download](https://github.com/AmanSikarwar/agent-skills-generator/releases/latest) |
| Windows | x86_64 | [Download](https://github.com/AmanSikarwar/agent-skills-generator/releases/latest) |

---

## Quick Start

### 1. Initialize Configuration

```bash
agent-skills-generator init
```

This creates a `skills.yaml` configuration file.

### 2. Crawl a Website

```bash
agent-skills-generator crawl https://docs.example.com
```

### 3. Find Your Skills

Generated skills are saved to `.agent/skills/` by default:

```
.agent/skills/
├── getting-started/
│   └── SKILL.md
├── api-reference/
│   └── SKILL.md
└── tutorials-basics/
    └── SKILL.md
```

Each `SKILL.md` contains:

```markdown
---
name: getting-started
description: Learn how to get started with our platform
metadata:
  url: https://docs.example.com/getting-started
---

# Getting Started

[Full documentation content converted to clean markdown...]
```

---

## Configuration

Create a `skills.yaml` file to customize crawling behavior:

```yaml
# Output directory for generated skills
output: .agent/skills

# Crawl settings
delay_ms: 100           # Delay between requests
max_depth: 25           # Maximum crawl depth
request_timeout_secs: 30
respect_robots_txt: true
subdomains: false
concurrency: 4          # Parallel page processing

# URL filtering rules
rules:
  # Only crawl documentation pages
  - url: "*/docs/*"
    action: allow

  # Ignore authentication pages
  - url: "*/login*"
    action: ignore
  - url: "*/auth/*"
    action: ignore

  # Ignore API internals
  - url: "*/api/internal/*"
    action: ignore

# CSS selectors for elements to remove
remove_selectors:
  - ".advertisement"
  - "#cookie-banner"
  - ".feedback-widget"
```

### Validate Configuration

```bash
agent-skills-generator validate --show
```

---

## Commands

| Command | Description |
|---------|-------------|
| `crawl <url>` | Crawl a website and generate skill files |
| `single <url>` | Process a single URL |
| `clean` | Remove generated skill files |
| `validate` | Validate configuration file |
| `init` | Create default configuration |

### Common Options

```bash
# Verbose output
agent-skills-generator -v crawl https://docs.example.com

# Very verbose (debug)
agent-skills-generator -vv crawl https://docs.example.com

# Custom config file
agent-skills-generator -c my-config.yaml crawl https://docs.example.com

# Custom output directory
agent-skills-generator -o ./my-skills crawl https://docs.example.com

# Limit pages crawled
agent-skills-generator crawl https://docs.example.com --max-pages 50

# Dry run (don't write files)
agent-skills-generator crawl https://docs.example.com --dry-run
```

---

## Examples

### Crawl Flutter Documentation

```bash
agent-skills-generator crawl https://docs.flutter.dev/ui
```

### Crawl Multiple URLs

```bash
agent-skills-generator crawl \
  https://docs.example.com/getting-started \
  https://docs.example.com/tutorials \
  https://docs.example.com/api
```

### Process Single Page

```bash
agent-skills-generator single https://docs.example.com/quick-start --stdout
```

### Resume Interrupted Crawl

```bash
agent-skills-generator crawl https://docs.example.com --resume
```

---

## How It Works

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Web Crawler   │────▶│ Content Cleaner │────▶│ Skill Generator │
│                 │     │                 │     │                 │
│ • Async crawl   │     │ • Remove nav    │     │ • YAML metadata │
│ • Robots.txt    │     │ • Remove ads    │     │ • Clean markdown│
│ • URL filtering │     │ • Remove noise  │     │ • Organized dirs│
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

1. **Crawl** — Discovers and fetches pages respecting robots.txt and rate limits
2. **Clean** — Removes navigation, scripts, styles, ads, and other noise
3. **Convert** — Transforms HTML to clean, token-efficient markdown
4. **Generate** — Creates structured SKILL.md files with metadata

---

## Use Cases

### AI Agent Enhancement

Give your AI agents access to up-to-date documentation:

```python
# Load skills into your agent's context
skills_dir = ".agent/skills"
for skill in load_skills(skills_dir):
    agent.add_context(skill)
```

### Documentation Indexing

Create searchable documentation archives:

```bash
agent-skills-generator crawl https://docs.company.com -o ./docs-archive
```

### Knowledge Base Generation

Build knowledge bases for RAG systems:

```bash
agent-skills-generator crawl https://wiki.example.com --max-depth 10
```

---

## Performance

Agent Skills Generator is built for speed:

| Metric | Value |
|--------|-------|
| Concurrent requests | Configurable (default: 4) |
| Memory usage | ~50MB typical |
| Pages/second | 10-50 (network dependent) |

---

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- crawl https://example.com

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">

**Built with Rust**

[Report Bug](https://github.com/AmanSikarwar/agent-skills-generator/issues) •
[Request Feature](https://github.com/AmanSikarwar/agent-skills-generator/issues)

</div>
