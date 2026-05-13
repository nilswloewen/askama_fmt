/// Line-by-line indentation state machine.
use crate::config::FormatOptions;
use crate::formatter::expand::BLOCK_HTML_TAGS;

/// HTML tags whose opening tag increases indent.
fn is_indent_html_tag(name: &str) -> bool {
    BLOCK_HTML_TAGS.contains(&name)
        && !is_void_html_tag(name)
        && name != "hr"
        && name != "br"
        && name != "link"
        && name != "meta"
}

fn is_void_html_tag(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

struct IndentState<'a> {
    opts: &'a FormatOptions,
    level: usize,
    in_raw: bool, // inside {% raw %} or <pre>/<script>/<style>
    raw_depth: usize,
    /// Non-None when we're inside a multi-line HTML opening tag whose `>` has
    /// not appeared yet (e.g. `<form\n  attr1\n  attr2>`).
    /// Tuple: (tag_name, is_block_level).  Block-level tags increment `level`
    /// when the closing `>` is found; inline/void tags do not.
    multi_line_tag: Option<(String, bool)>,
    /// Stack of base indent levels for open `custom_blocks` tags.
    /// Used by `custom_blocks_branch` keywords to reset each branch to the
    /// same indentation level inside the enclosing block.
    block_base_levels: Vec<usize>,
}

impl<'a> IndentState<'a> {
    fn new(opts: &'a FormatOptions) -> Self {
        Self {
            opts,
            level: 0,
            in_raw: false,
            raw_depth: 0,
            multi_line_tag: None,
            block_base_levels: Vec::new(),
        }
    }

    fn indent(&self) -> String {
        " ".repeat(self.opts.indent * self.level)
    }

    fn indent_at(&self, level: usize) -> String {
        " ".repeat(self.opts.indent * level)
    }

    /// Indentation for continuation attribute lines inside a multi-line tag.
    /// Aligns to the column after `<tagname `.
    fn continuation_indent(&self, tag_name: &str) -> String {
        " ".repeat(self.opts.indent * self.level + 1 + tag_name.len() + 1)
    }
}

pub fn indent(html: &str, opts: &FormatOptions) -> String {
    let mut state = IndentState::new(opts);
    let mut out = String::with_capacity(html.len());


    for line in html.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }

        // --- Multi-line HTML opening tag continuation ---
        // Covers both block-level tags (<form>, <div>, …) and inline/void
        // tags (<input>, <a>, …) that have their `>` on a later line.
        if let Some((ref tag_name, is_block)) = state.multi_line_tag.clone() {
            let cont_indent = state.continuation_indent(tag_name);
            if html_open_tag_closes_here(trimmed) {
                // This line has the closing `>` — tag is fully open.
                state.multi_line_tag = None;
                out.push_str(&cont_indent);
                out.push_str(trimmed);
                out.push('\n');
                // Block-level tags open an indent level; inline/void do not.
                // Self-closing (`/>`) also never opens a level.
                if is_block && !trimmed.trim_end().ends_with("/>") {
                    state.level += 1;
                }
            } else {
                // Continuation attribute line (no `>` yet).
                out.push_str(&cont_indent);
                out.push_str(trimmed);
                out.push('\n');
            }
            continue;
        }

        // Inside a raw/verbatim block: emit as-is
        if state.in_raw {
            if is_raw_block_close(trimmed) {
                state.raw_depth = state.raw_depth.saturating_sub(1);
                if state.raw_depth == 0 {
                    state.in_raw = false;
                    // Emit the closing line.  If the closing tag is the entire
                    // trimmed line (e.g. `</style>`) give it current-level
                    // indentation.  When it's embedded at the end of content
                    // (e.g. `}</style>`) keep it as-is so that the output is
                    // stable across multiple formatter passes.
                    let starts_with_close = trimmed.starts_with("</style>")
                        || trimmed.starts_with("</script>")
                        || trimmed.starts_with("</pre>")
                        || trimmed.starts_with("{%");
                    if starts_with_close {
                        out.push_str(&state.indent());
                    }
                    out.push_str(trimmed);
                    out.push('\n');
                } else {
                    out.push_str(trimmed);
                    out.push('\n');
                }
            } else {
                out.push_str(trimmed);
                out.push('\n');
            }
            continue;
        }

        // Detect raw block opening
        if is_raw_block_open(trimmed) {
            state.in_raw = true;
            state.raw_depth = 1;
            out.push_str(&state.indent());
            out.push_str(trimmed);
            out.push('\n');
            continue;
        }

        // --- Classify the line ---

        // 1. HTML closing tag at start of line → unindent before printing
        if let Some(tag) = parse_html_close_tag(trimmed) {
            if is_indent_html_tag(&tag) {
                state.level = state.level.saturating_sub(1);
                out.push_str(&state.indent());
                out.push_str(trimmed);
                out.push('\n');
                // If the same line also has an open tag (e.g. </td><td>), handle that
                continue;
            }
        }

        // 2. Template tag classification
        if let Some(kw) = parse_template_keyword(trimmed) {
            // 2a. Branch keyword ("when"): resets to the enclosing match's base
            // level + 1, then pushes for content.
            if BRANCH_KEYWORDS.contains(&kw.as_str()) {
                if let Some(&base) = state.block_base_levels.last() {
                    state.level = base + 1;
                    out.push_str(&state.indent());
                    out.push_str(trimmed);
                    out.push('\n');
                    state.level = base + 2;
                } else {
                    // No enclosing custom block — no-change fallback
                    out.push_str(&state.indent());
                    out.push_str(trimmed);
                    out.push('\n');
                }
                continue;
            }

            // 2b. Branch-aware end keyword ("endmatch"):
            // pops back to the base level recorded when the block was opened.
            if BRANCH_END_KEYWORDS.contains(&kw.as_str()) {
                let base = state
                    .block_base_levels
                    .pop()
                    .unwrap_or_else(|| state.level.saturating_sub(1));
                state.level = base;
                out.push_str(&state.indent());
                out.push_str(trimmed);
                out.push('\n');
                continue;
            }

            // 2c. Built-in closing tag (`{% endif %}`, `{% endfor %}`, …)
            if UNINDENT_KEYWORDS.contains(&kw.as_str()) {
                state.level = state.level.saturating_sub(1);
                out.push_str(&state.indent());
                out.push_str(trimmed);
                out.push('\n');
                continue;
            }

            // 3. Unindent-line tags (else, else if) → print at level-1
            if UNINDENT_LINE_KEYWORDS.contains(&kw.as_str()) {
                let effective = state.level.saturating_sub(1);
                out.push_str(&state.indent_at(effective));
                out.push_str(trimmed);
                out.push('\n');
                continue;
            }

            // 4. Tags with no indent change (let, call, import, include, extends, …)
            if NO_CHANGE_KEYWORDS.contains(&kw.as_str()) {
                out.push_str(&state.indent());
                out.push_str(trimmed);
                out.push('\n');
                continue;
            }

            // 5. Indent-opening template tag
            if INDENT_KEYWORDS.contains(&kw.as_str()) {
                // Track base level for match so the "when" branch keyword can reset correctly
                if kw == "match" {
                    state.block_base_levels.push(state.level);
                }
                out.push_str(&state.indent());
                out.push_str(trimmed);
                out.push('\n');
                state.level += 1;
                continue;
            }
        }

        // 6. HTML opening tag → check if it increases indent
        if let Some((open_tag, is_self_closing, has_close_on_same_line)) =
            parse_html_open_tag(trimmed)
        {
            if is_indent_html_tag(&open_tag) && !is_self_closing {
                // If the closing tag is also on this same line (e.g. <td>val</td>),
                // don't change the indent level.
                if has_close_on_same_line {
                    out.push_str(&state.indent());
                    out.push_str(trimmed);
                    out.push('\n');
                    continue;
                }

                let formatted = maybe_format_attributes(trimmed, state.level, opts);
                out.push_str(&state.indent());
                out.push_str(&formatted);
                out.push('\n');
                state.level += 1;
                continue;
            }
        }

        // 6b. HTML tag that doesn't close on this line — covers both block-level
        //     tags (<form>, <table>, …) and inline/void tags (<input>, <a>, …)
        //     so that continuation attribute lines get the correct alignment
        //     regardless of whether the tag opens an indent level.
        if let Some((tag_name, is_block)) = parse_unclosed_html_open_tag(trimmed) {
            out.push_str(&state.indent());
            out.push_str(trimmed);
            out.push('\n');
            state.multi_line_tag = Some((tag_name, is_block));
            continue;
        }

        // 7. Default: emit at current indent level with attribute formatting
        let formatted = maybe_format_attributes(trimmed, state.level, opts);
        out.push_str(&state.indent());
        out.push_str(&formatted);
        out.push('\n');
    }

    // Ensure single trailing newline
    let result = out.trim_end_matches('\n').to_string();
    result + "\n"
}

// ── Keyword classification ──────────────────────────────────────────────────

const INDENT_KEYWORDS: &[&str] =
    &["if", "for", "macro", "block", "filter", "with", "raw", "match"];

const UNINDENT_KEYWORDS: &[&str] =
    &["endif", "endfor", "endmacro", "endblock", "endfilter", "endwith", "endraw"];

const UNINDENT_LINE_KEYWORDS: &[&str] = &["else", "else if"];

const BRANCH_KEYWORDS: &[&str] = &["when"];

const BRANCH_END_KEYWORDS: &[&str] = &["endmatch"];

const NO_CHANGE_KEYWORDS: &[&str] = &["let", "call", "import", "include", "extends"];

// ── Tag parsers ─────────────────────────────────────────────────────────────

/// If line starts with `</tag`, return `tag` (lowercased).
fn parse_html_close_tag(line: &str) -> Option<String> {
    let s = line.trim_start();
    if !s.starts_with("</") {
        return None;
    }
    let rest = &s[2..];
    let end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '-')
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some(rest[..end].to_lowercase())
}

/// Extract the keyword from a template tag line, e.g. `{% when Some with (x) %}` → `"when"`.
/// Also handles `{%- when -%}` whitespace-stripped variants.
fn parse_template_keyword(line: &str) -> Option<String> {
    let s = line.trim();
    if !s.starts_with("{%") {
        return None;
    }
    let inner = s[2..].trim_start_matches(['-', '+', '~', ' ', '\t']);
    // "else if" is a two-word keyword
    if inner.starts_with("else if") {
        return Some("else if".to_string());
    }
    let kw: String = inner
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(['-', '+', '~'])
        .to_string();
    if kw.is_empty() {
        None
    } else {
        Some(kw)
    }
}

/// Returns `(tag_name, is_self_closing, has_matching_close_on_same_line)`.
fn parse_html_open_tag(line: &str) -> Option<(String, bool, bool)> {
    let s = line.trim_start();
    if !s.starts_with('<') || s.starts_with("</") || s.starts_with("<!") || s.starts_with("<?") {
        return None;
    }
    let rest = &s[1..];
    let end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '-')
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let tag = rest[..end].to_lowercase();

    // Use the shared scanner — correctly skips {%...%} containing `>`.
    let close_pos = super::find_html_tag_close(s)?;

    // Self-closing: the byte immediately before `>` is `/`.
    let self_closing = close_pos > 0 && s.as_bytes()[close_pos - 1] == b'/';

    // Check if there's a matching close tag after the opening tag.
    let after_open = &s[close_pos + 1..];
    let close_tag = format!("</{}", tag);
    let has_close = after_open.to_lowercase().contains(&close_tag);

    Some((tag, self_closing, has_close))
}

fn is_raw_block_open(line: &str) -> bool {
    let s = line.trim();
    // Only open a raw block if the closing tag is NOT also on this line.
    for (open, close) in &[
        ("<pre", "</pre>"),
        ("<script", "</script>"),
        ("<style", "</style>"),
    ] {
        if s.starts_with(open) && !s.contains(close) {
            return true;
        }
    }
    if let Some(kw) = parse_template_keyword(s) {
        return kw == "raw";
    }
    false
}

/// If the line opens an HTML tag whose `>` is NOT on this line, returns
/// `(tag_name, is_block_level)`.  Works for both block-level tags (<form>…)
/// and inline/void tags (<input>, <a>…) so that multi-line attribute
/// continuation lines are always aligned correctly.
fn parse_unclosed_html_open_tag(line: &str) -> Option<(String, bool)> {
    let s = line.trim_start();
    if !s.starts_with('<') || s.starts_with("</") || s.starts_with("<!") || s.starts_with("<?") {
        return None;
    }
    let rest = &s[1..];
    let end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '-')
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let tag = rest[..end].to_lowercase();
    // Use the shared scanner — `{%...%}` containing `>` is correctly skipped.
    if super::find_html_tag_close(s).is_some() {
        return None;
    }
    let is_block = is_indent_html_tag(&tag);
    Some((tag, is_block))
}

/// Returns true if `line` closes a pending multi-line HTML opening tag, i.e.
/// it contains an unquoted `>` outside any template tag or quoted value.
fn html_open_tag_closes_here(line: &str) -> bool {
    super::find_html_tag_close(line.trim()).is_some()
}

fn is_raw_block_close(line: &str) -> bool {
    let s = line.trim();
    // The closing tag can appear anywhere on the line (e.g. `}</style>`).
    if s.contains("</pre>") || s.contains("</script>") || s.contains("</style>") {
        return true;
    }
    if let Some(kw) = parse_template_keyword(s) {
        return kw == "endraw";
    }
    false
}

// ── Attribute formatting ─────────────────────────────────────────────────────

pub fn maybe_format_attributes(line: &str, level: usize, opts: &FormatOptions) -> String {
    let s = line.trim();
    if !s.starts_with('<') || s.starts_with("</") || s.starts_with("<!") {
        return s.to_string();
    }

    let rest = &s[1..];
    let name_end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '-')
        .unwrap_or(rest.len());
    let tag_name = &rest[..name_end];

    if !s[1 + name_end..].starts_with(|c: char| c.is_whitespace()) {
        return s.to_string();
    }

    let (tag_only, after_close) = split_tag_from_content(s);
    let attrs = parse_attributes(tag_only);
    if attrs.len() < 2 {
        return s.to_string();
    }


    // Sort attributes alphabetically (unhinged default: on).
    // Skip if template syntax is present — reordering {% if %}...{% endif %}
    // conditional attributes would break semantics.
    let attrs = if opts.sort_attributes {
        sort_attributes(attrs)
    } else {
        attrs
    };

    let is_self_closing = tag_only.trim_end().ends_with("/>");
    let close = if is_self_closing { " />" } else { ">" };

    // Reconstruct the tag with sorted attributes and check line length.
    let tag_sorted = format!("<{} {}{}", tag_name, attrs.join(" "), close);
    let indent_len = opts.indent * level;
    if indent_len + tag_sorted.len() <= opts.max_line_length {
        return if after_close.is_empty() {
            tag_sorted
        } else {
            format!("{}{}", tag_sorted, after_close)
        };
    }

    // Break: align subsequent attributes under the first attribute column.
    let align = " ".repeat(indent_len + 1 + tag_name.len() + 1);
    let mut out_lines: Vec<String> = attrs
        .iter()
        .enumerate()
        .map(|(i, attr)| {
            if i == 0 {
                format!("<{} {}", tag_name, attr)
            } else {
                format!("{}{}", align, attr)
            }
        })
        .collect();

    if let Some(last) = out_lines.last_mut() {
        last.push_str(close);
        if !after_close.is_empty() {
            last.push_str(after_close);
        }
    }
    out_lines.join("\n")
}


/// Split an HTML tag string into the `<tag attrs>` portion and anything after `>`.
/// e.g. `<a href="x">text</a>` → (`<a href="x">`, `text</a>`)
fn split_tag_from_content(s: &str) -> (&str, &str) {
    match super::find_html_tag_close(s) {
        Some(pos) => (&s[..pos + 1], &s[pos + 1..]),
        None => (s, ""),
    }
}

/// Very simple attribute parser: splits on whitespace boundaries respecting quotes
/// and template tags `{{ }}` / `{% %}`.
fn parse_attributes(tag: &str) -> Vec<String> {
    let start = tag.find(|c: char| c.is_whitespace()).unwrap_or(tag.len());
    // Use the shared scanner for `>` — handles {%...%} with `>` inside and
    // multi-byte characters (returns a correct byte offset).
    let end = super::find_html_tag_close(&tag[start..])
        .map(|rel| start + rel)
        .unwrap_or(tag.len());

    // Strip trailing ` /` from self-closing tags so the `/` is not treated as
    // a separate attribute token.
    let attrs_raw = &tag[start..end];
    let attrs_str = attrs_raw.trim_end_matches('/').trim_end();
    split_attrs(attrs_str)
}

/// Sort attributes alphabetically by name. Skips reordering if any attribute
/// contains template syntax (`{%` or `{{`) — those are conditional attribute
/// injections whose relative order is load-bearing.
fn sort_attributes(mut attrs: Vec<String>) -> Vec<String> {
    if attrs.iter().any(|a| a.contains("{%") || a.contains("{{")) {
        return attrs;
    }
    attrs.sort_by(|a, b| {
        let ka = a.split('=').next().unwrap_or(a).trim();
        let kb = b.split('=').next().unwrap_or(b).trim();
        ka.to_lowercase().cmp(&kb.to_lowercase())
    });
    attrs
}

fn split_attrs(s: &str) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut current = String::new();
    let mut in_q: Option<char> = None;
    let mut depth_tmpl = 0usize;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match in_q {
            Some(q) if c == q => {
                current.push(c);
                in_q = None;
            }
            Some(_) => {
                current.push(c);
            }
            None => {
                if c == '{'
                    && chars
                        .get(i + 1)
                        .copied()
                        .is_some_and(|n| n == '{' || n == '%')
                {
                    depth_tmpl += 1;
                    current.push(c);
                } else if depth_tmpl > 0 && (c == '}' || c == '%') && chars.get(i + 1) == Some(&'}')
                {
                    depth_tmpl -= 1;
                    current.push(c);
                } else if depth_tmpl == 0 && c.is_whitespace() {
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        attrs.push(trimmed);
                    }
                    current.clear();
                } else {
                    if c == '"' || c == '\'' {
                        in_q = Some(c);
                    }
                    current.push(c);
                }
            }
        }
        i += 1;
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        attrs.push(trimmed);
    }
    attrs
}
