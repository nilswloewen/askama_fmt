# Changelog

## [0.3.3] - 2026-06-03

### Features

- `{# askama_fmt: skip-file #}` directive — anywhere in a template, returns the
  file byte-for-byte unchanged.
- `{# askama_fmt: off #}` / `{# askama_fmt: on #}` directives — preserve the
  region between them verbatim while the rest of the file is formatted normally.
  Whitespace-control marks (`{#- … -#}`) are tolerated on both forms.

## [0.3.0] - 2026-05-13

### Breaking changes

- `FormatOptions` reduced to three fields: `indent`, `max_line_length`, `sort_attributes`.
  All template-syntax fields (`custom_blocks`, `custom_blocks_unindent_line`, `ignore_blocks`,
  `preserve_blank_lines`, `max_blank_lines`, `max_attribute_length`) removed.
- `{% match %}`/`{% when %}`/`{% endmatch %}`, `{% call %}`, and all other Askama keywords
  are now hardcoded — zero configuration required for standard Askama templates.
- Attribute breaking threshold is now `max_line_length` (the single ruler for everything).
- Blank lines always collapse to at most one (rustfmt-style, hardcoded).

### Features

- `sort_attributes` option (default `false`): opt-in alphabetical attribute sorting.
- `{% when %}` branches indented inside `{% match %}` (Rust-style match arms).
- `<label>` and `<span>` added to block HTML tags.

### Performance

- All keyword lists converted from heap-allocated `Vec<String>` per call to `const &[&str]` slices.
- Expand pass: `Vec<char>` + per-`<` string allocations eliminated; byte-index scanning throughout.
- Indent pass: indent strings written directly to output buffer (`std::iter::repeat_n`), no intermediate `String` allocation per line.
- `parse_template_keyword`, `parse_html_close_tag`, `parse_html_open_tag` return `&str` slices borrowing from the input — no heap allocation per tag.
- `contains_close_tag` uses byte-level `windows` + `eq_ignore_ascii_case` instead of `to_lowercase().contains()`.

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
