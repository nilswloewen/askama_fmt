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

4. **indent** (`indent.rs`) — Line-by-line state machine. Tracks indent level, raw-block state (`<pre>`, `<script>`, `<style>`, `{% raw %}`), multi-line HTML opening tags, and a `block_base_levels` stack for branch-aware blocks. Template keywords are classified into five buckets:
   - **branch** (`when` — hardcoded): resets to the enclosing `custom_blocks` base level + 1 and pushes for content; `block_base_levels` stack tracks the base level of each open `custom_blocks` tag
   - **branch-end** (end forms of `custom_blocks`, e.g. `endmatch`): pops `block_base_levels` to restore the block's opening level
   - **indent-opening** (`if`, `for`, `macro`, … + `custom_blocks`): prints at current level, pushes +1
   - **unindent-closing** (`endif`, `endfor`, … built-ins): pops −1 then prints
   - **unindent-line** (`else`, `else if` + `custom_blocks_unindent_line`): prints at level−1, level unchanged
   - **no-change** (`let`, `call`, `include`, … + `ignore_blocks`): prints at current level, level unchanged

5. **condense** (`condense.rs`) — Collapses short tag pairs back onto one line if the result fits within `max_line_length`. Two sub-passes: HTML pairs first, then template pairs.

### Shared `find_html_tag_close`

`formatter/mod.rs::find_html_tag_close()` is used by every pass to locate the `>` that closes an HTML opening tag. It's necessary because naively scanning for `>` breaks on `{% if x > 0 %}` inside attributes. The scanner skips over `{%...%}` and `{{...}}` delimiters and respects quoted attribute values.

### Configuration

`config.rs::FormatOptions` deserializes from `askama_fmt.toml`. Config is discovered by walking up from the target file's directory until hitting `.git` or the filesystem root. CLI flags are applied on top as overrides via `apply_overrides`.

`{% match %}`/`{% when %}`/`{% endmatch %}` are hardcoded — no configuration needed. The two remaining list options (`custom_blocks_unindent_line`, `ignore_blocks`) extend the built-in keyword sets at runtime.

### Regex strategy

- `fancy-regex` (compress pass only) — supports lookbehind, used for the multi-line tag flattening regex.
- `regex` (condense pass) — faster standard crate, sufficient for the collapse patterns.
- Both use `OnceLock<Regex>` so regexes are compiled once and reused.

### Tests

All tests are integration tests in `tests/askama.rs`. They call `format()` directly with inline input/expected strings. The shared `opts()` helper creates `FormatOptions` with `ignore_blocks = ["call"]` — `{% match %}`/`{% when %}` need no configuration.
