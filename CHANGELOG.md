# Changelog

## [0.2.0] - 2026-05-13

### Breaking changes

- `{% when %}` is now a hardcoded branch keyword — it is automatically indented
  inside `{% match %}` blocks without any configuration. Remove
  `custom_blocks_unindent_line = ["when"]` from your `askama_fmt.toml`.

### Features

- `{% when %}` branches are indented inside `{% match %}`: each arm prints one
  level in from the `{% match %}`, with content indented one level further.
- `<label>` and `<span>` added to the block HTML tag list — content inside these
  elements is now indented.

### Performance

- Expand pass rewritten from O(n²) to O(n): raw spans are pre-computed in a
  single scan and checked with binary search instead of rescanning from byte 0
  on every tag match. `break_html_tags` now uses byte indices directly,
  eliminating O(n) string allocations per `<` character.

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
