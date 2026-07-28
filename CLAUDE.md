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

1. **compress** (`compress.rs`) — Flattens multi-line HTML opening tags to a single line using `fancy-regex`. Skips tags whose attribute list contains `{%...%}` (those must stay multi-line for the expand pass to work correctly), and skips any match starting inside a template construct.

2. **expand** (`expand.rs`) — Puts block-level HTML tags and Askama template tags each on their own line. Runs two sub-passes: HTML block tags first, then template tags. The HTML sub-pass copies `{% %}` / `{{ }}` / `{# #}` through verbatim — their contents are Rust, not markup. Template tags inside HTML attribute lists (conditional attrs like `{% if cond %}class="x"{% endif %}`) are intentionally kept inline. Raw spans (`{# #}`, `<!-- -->`, `{% raw %}`, and each of `RAW_CONTENT_TAGS`) are pre-computed once per pass via `compute_raw_spans` and checked with `is_in_raw_span` (binary search) — previously O(n) per tag, now O(log k).

3. **clean_whitespace** (`condense.rs`) — Strips trailing whitespace and collapses excess blank lines. Runs before indent so the indenter sees clean input.

4. **indent** (`indent.rs`) — Line-by-line state machine. Tracks indent level, raw-block state (`expand::RAW_CONTENT_TAGS` = `<pre>`, `<script>`, `<style>`, `<textarea>`, plus `{% raw %}`) — inside a raw block every line is emitted **verbatim**, and `</pre>` / `</textarea>` are not re-indented because the whitespace before them is part of the element's value, multi-line HTML opening tags, multi-line Askama tags (a macro signature broken over several lines), and a `block_base_levels` stack for branch-aware blocks. `classify()` maps each hardcoded keyword to an `Effect`, applied as `open_effect` (before printing) + `close_effect` (after):
   - **Branch** (`when`): resets to the enclosing match's base level + 1, pushes for content; `block_base_levels` stack tracks the base level of each open `match`
   - **BranchInnerEnd** (`endwhen`): closes one arm, back to its `{% when %}` level
   - **BranchEnd** (`endmatch`): pops `block_base_levels` to restore the match's opening level
   - **Indent** (`if`, `for`, `macro`, `block`, `filter`, `with`, `raw`, `match`): prints at current level, pushes +1
   - **Unindent** (`endif`, `endfor`, `endmacro`, `endcall`, `endlet`, `endset`, …): pops −1 then prints
   - **UnindentLine** (`else`, `else if`, `elif`): prints at level−1, level unchanged
   - **NoChange** (`include`, `import`, `extends`, `break`, `continue`, `mut`, `decl`, `declare`): prints at current level, level unchanged

   `call`, `let` and `set` are classified at runtime rather than by a fixed bucket — see `BlockPairs` below.

5. **condense** (`condense.rs`) — Collapses short tag pairs back onto one line if the result fits within `max_line_length`. Each round runs HTML pairs then template pairs, and rounds repeat to a fixed point (collapsing `{% call x() %}{% endcall %}` can bring a wrapping `<span>` within reach). A body is only collapsed when the result is genuinely one line; raw-content tags (`script`, `style`, `pre`) are the exception, where pulling the first line back up restores what the author wrote.

### Shared helpers in `formatter/mod.rs`

- `find_html_tag_close()` locates the `>` that closes an HTML opening tag. Necessary because naively scanning for `>` breaks on `{% if x > 0 %}` inside attributes. Skips `{%...%}` / `{{...}}` and respects quoted attribute values.
- `template_spans()` + `in_span()` give the byte ranges of every `{% %}` / `{{ }}` / `{# #}`. Passes that scan for HTML must treat these as opaque — askama 0.16 macro type hints (`Vec<Td>`, `Option<Body>`) otherwise read as HTML tags and get rewritten.
- `leading_keyword()` / `tag_keyword()` extract a tag's keyword, stopping at the first non-identifier byte so the caller-args form `{% call(item) each(items) %}` still yields `call`. `else if` is the one two-word keyword.
- `let_opens_block()` distinguishes `{% let x = 1 %}` (statement) from `{% let x %}` (block, closed by `{% endlet %}`) — the block form is the one with no `=`.
- `BlockPairs` counts `endcall` / `endlet` / `endset` up front, and each opener claims one. An opener with no closer left is the askama ≤ 0.15 statement form (`{% call foo() %}`, `{% let x %}` forward declaration) and must not shift the indent level. Both `expand` and `indent` use this so their decisions agree.

### Configuration

`config.rs::FormatOptions` deserializes from `askama_fmt.toml`. Config is discovered by walking up from the target file's directory until hitting `.git` or the filesystem root. CLI flags are applied on top as overrides via `apply_overrides`.

`FormatOptions` has three fields: `indent`, `max_line_length`, and `sort_attributes` (bool, default `true` — alphabetizes HTML attributes; skipped for tags containing template syntax). Both attribute breaking and tag-pair collapsing use `max_line_length` as the single ruler. Blank lines are always collapsed to at most one. All Askama template syntax is hardcoded — zero syntax configuration required.

### Regex strategy

- `fancy-regex` (compress pass only) — supports lookbehind, used for the multi-line tag flattening regex.
- `regex` (condense pass) — faster standard crate, sufficient for the collapse patterns.
- Both use `OnceLock<Regex>` so regexes are compiled once and reused.

### Tests

All tests are integration tests in `tests/askama.rs` (plus a few unit tests in `formatter/skip.rs`). They call `format()` directly with inline input/expected strings. The shared `opts()` helper is just `FormatOptions { indent: 4, ..Default::default() }` — no syntax configuration needed.

When changing template-syntax handling, check output against the real parser rather than by eye: `askama_parser` (same version as askama) exposes `Ast::from_str(src, None, &Syntax::default())`, so a throwaway binary can confirm that a formatted template still parses and that a syntax form is actually valid. It is deliberately **not** a dev-dependency — askama 0.16 needs Rust 1.88, above this crate's MSRV.
