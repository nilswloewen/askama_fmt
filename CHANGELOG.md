# Changelog

## [0.1.0] - 2026-05-12

Initial release.

### Features

- Four-pass formatting pipeline: compress → expand → indent → condense
- First-class support for Askama template syntax: `{% match %}`/`{% when %}`, `{% call %}`, `{% if let %}`, `{% let %}`, `{% for %}`, `{% filter %}`, `{% macro %}`
- Config file (`askama_fmt.toml`) with support for `custom_blocks`, `custom_blocks_unindent_line`, and `ignore_blocks`
- CLI with `--check` and `--diff` modes for CI integration
- `--stdin-filepath` mode for editor integration
- Parallel file processing via Rayon
- Respects `.gitignore` when walking directories
