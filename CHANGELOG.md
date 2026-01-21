# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-01-21

### Added

- **Multi-IDE Target Support**: Added support for configuring skills across multiple IDEs and AI agents:
  - VS Code Copilot
  - Claude Code
  - Cursor
  - Antigravity
  - OpenAI Codex
  - OpenCode
- New `--target` flag to specify the desired IDE/agent
- New `--user` flag for user-level configuration (vs project-level)
- Automatic path resolution based on target IDE and scope
- **Interactive Configuration Wizard**: Added an interactive initialization mode using the `inquire` crate
  - Prompts users for target IDE selection, configuration scope, output path, and crawl settings
  - Support for `--non-interactive` mode for CI/CD pipelines and scripting
- Added `CLAUDE.md` for project context documentation

### Fixed

- Added `restore-keys` for cache fallback in CI workflow to improve cache hit rates
- Added OpenSSL cross-compilation support for release builds

## [0.1.0] - 2026-01-20

### Added

- Initial release of agent-skills-generator
- Core functionality to crawl repositories and generate agent skills files
- Content processor module for parsing and extracting documentation
- Installation scripts for Windows and Unix-based systems
- CI and release workflows for automated testing and deployment
- README.md with project overview, features, installation, and usage instructions

[0.2.0]: https://github.com/AmanSikarwar/agent-skills-generator/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/AmanSikarwar/agent-skills-generator/releases/tag/v0.1.0
