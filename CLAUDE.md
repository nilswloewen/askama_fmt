# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# Run a specific test by name
cargo test match_basic

# Lint + format + docs (mirrors `just clean`)
cargo clippy --all-targets
cargo fmt
cargo doc --no-deps

# Verify publish readiness
cargo publish --dry-run --allow-dirty

# Find true MSRV
cargo msrv find
```

## Architecture

The crate is both a library (`src/lib.rs` → `format()` + `FormatOptions`) and a CLI binary (`src/main.rs`). The binary handles file collection (glob/directory walking via `ignore`), parallel processing via Rayon, and `--check`/`--diff` modes.

File extension filtering only applies to **directory** walking — only `*.askama.html` files are discovered. When given a direct **file path or glob**, any extension is accepted and formatted.

### Formatting pipeline

`formatter/mod.rs::format()` runs five sequential passes on the template string:

1. **compress** (`compress.rs`) — Flattens multi-line HTML opening tags to a single line using `fancy-regex`. Skips tags whose attribute list contains `{%...%}` (those must stay multi-line for the expand pass to work correctly).

2. **expand** (`expand.rs`) — Puts block-level HTML tags and Askama template tags each on their own line. Runs two sub-passes: HTML block tags first, then template tags. Template tags inside HTML attribute lists (conditional attrs like `{% if cond %}class="x"{% endif %}`) are intentionally kept inline. Raw spans (`{# #}`, `<!-- -->`, `{% raw %}`, `<pre>`, `<script>`, `<style>`) are pre-computed once per pass via `compute_raw_spans` and checked with `is_in_raw_span` (binary search) — previously O(n) per tag, now O(log k).

3. **clean_whitespace** (`condense.rs`) — Strips trailing whitespace and collapses excess blank lines. Runs before indent so the indenter sees clean input.

4. **indent** (`indent.rs`) — Line-by-line state machine. Tracks indent level, raw-block state (`<pre>`, `<script>`, `<style>`, `{% raw %}`), multi-line HTML opening tags, and a `block_base_levels` stack for branch-aware blocks. All Askama keywords are hardcoded into five buckets:
   - **branch** (`when`): resets to the enclosing match's base level + 1, pushes for content; `block_base_levels` stack tracks the base level of each open `match`
   - **branch-end** (`endmatch`): pops `block_base_levels` to restore the match's opening level
   - **indent-opening** (`if`, `for`, `macro`, `block`, `filter`, `with`, `raw`, `match`): prints at current level, pushes +1
   - **unindent-closing** (`endif`, `endfor`, `endmacro`, …): pops −1 then prints
   - **unindent-line** (`else`, `else if`): prints at level−1, level unchanged
   - **no-change** (`let`, `call`, `include`, `import`, `extends`): prints at current level, level unchanged

5. **condense** (`condense.rs`) — Collapses short tag pairs back onto one line if the result fits within `max_line_length`. Two sub-passes: HTML pairs first, then template pairs.

### Shared `find_html_tag_close`

`formatter/mod.rs::find_html_tag_close()` is used by every pass to locate the `>` that closes an HTML opening tag. It's necessary because naively scanning for `>` breaks on `{% if x > 0 %}` inside attributes. The scanner skips over `{%...%}` and `{{...}}` delimiters and respects quoted attribute values.

### Configuration

`config.rs::FormatOptions` deserializes from `askama_fmt.toml`. Config is discovered by walking up from the target file's directory until hitting `.git` or the filesystem root. CLI flags are applied on top as overrides via `apply_overrides`.

`FormatOptions` has three fields: `indent`, `max_line_length`, and `sort_attributes` (bool, default `true` — alphabetizes HTML attributes; skipped for tags containing template syntax). Both attribute breaking and tag-pair collapsing use `max_line_length` as the single ruler. Blank lines are always collapsed to at most one. All Askama template syntax is hardcoded — zero syntax configuration required.

### Regex strategy

- `fancy-regex` (compress pass only) — supports lookbehind, used for the multi-line tag flattening regex.
- `regex` (condense pass) — faster standard crate, sufficient for the collapse patterns.
- Both use `OnceLock<Regex>` so regexes are compiled once and reused.

### Tests

All tests are integration tests in `tests/askama.rs`. They call `format()` directly with inline input/expected strings. The shared `opts()` helper is just `FormatOptions { indent: 4, ..Default::default() }` — no syntax configuration needed.
