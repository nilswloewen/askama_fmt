/// Line-by-line indentation state machine.
use crate::config::FormatOptions;
use crate::formatter::expand::{BLOCK_HTML_TAGS, RAW_CONTENT_TAGS};
use crate::formatter::BlockPairs;

const VOID_HTML_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

fn is_void_html_tag(name: &str) -> bool {
    VOID_HTML_TAGS.iter().any(|&t| t.eq_ignore_ascii_case(name))
}

/// HTML tags whose opening tag increases indent.
fn is_indent_html_tag(name: &str) -> bool {
    BLOCK_HTML_TAGS
        .iter()
        .any(|&t| t.eq_ignore_ascii_case(name))
        && !is_void_html_tag(name)
        && !"hr".eq_ignore_ascii_case(name)
        && !"br".eq_ignore_ascii_case(name)
        && !"link".eq_ignore_ascii_case(name)
        && !"meta".eq_ignore_ascii_case(name)
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
    /// Non-None when we're inside an Askama tag whose `%}` has not appeared yet
    /// (e.g. a macro definition with one typed argument per line).
    /// Tuple: (effect, is_match, level the opening line was printed at).
    multi_line_template: Option<(Effect, bool, usize)>,
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
            multi_line_template: None,
            block_base_levels: Vec::new(),
        }
    }

    /// Level adjustments that happen *before* a template tag is printed.
    /// Returns the level the tag itself should be printed at.
    fn open_effect(&mut self, effect: Effect) -> usize {
        match effect {
            Effect::Indent | Effect::NoChange => self.level,
            Effect::Unindent => {
                self.level = self.level.saturating_sub(1);
                self.level
            }
            // The body stays where it is; only this one line moves out.
            Effect::UnindentLine => self.level.saturating_sub(1),
            Effect::Branch => match self.block_base_levels.last() {
                Some(&base) => {
                    self.level = base + 1;
                    self.level
                }
                // No enclosing `{% match %}` — leave the level alone.
                None => self.level,
            },
            Effect::BranchInnerEnd => match self.block_base_levels.last() {
                Some(&base) => {
                    self.level = base + 1;
                    self.level
                }
                None => {
                    self.level = self.level.saturating_sub(1);
                    self.level
                }
            },
            Effect::BranchEnd => {
                let base = self
                    .block_base_levels
                    .pop()
                    .unwrap_or_else(|| self.level.saturating_sub(1));
                self.level = base;
                self.level
            }
        }
    }

    /// Level adjustments that happen *after* a template tag is printed.
    fn close_effect(&mut self, effect: Effect, is_match: bool) {
        match effect {
            Effect::Indent => {
                // Record where the `{% match %}` sits so its `{% when %}` arms
                // and `{% endmatch %}` can snap back to it.
                if is_match {
                    self.block_base_levels.push(self.level);
                }
                self.level += 1;
            }
            // Without an enclosing `{% match %}` the arm was printed in place,
            // so there is nothing to indent into.
            Effect::Branch if !self.block_base_levels.is_empty() => {
                self.level += 1;
            }
            _ => {}
        }
    }

    fn write_indent(&self, out: &mut String) {
        out.extend(std::iter::repeat_n(' ', self.opts.indent * self.level));
    }

    fn write_indent_at(&self, out: &mut String, level: usize) {
        out.extend(std::iter::repeat_n(' ', self.opts.indent * level));
    }

    /// Indentation for continuation attribute lines inside a multi-line tag.
    /// Aligns to the column after `<tagname `.
    fn write_continuation_indent(&self, out: &mut String, tag_name: &str) {
        let n = self.opts.indent * self.level + 1 + tag_name.len() + 1;
        out.extend(std::iter::repeat_n(' ', n));
    }
}

pub fn indent(html: &str, opts: &FormatOptions) -> String {
    let mut state = IndentState::new(opts);
    let mut pairs = BlockPairs::scan(html);
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
            if html_open_tag_closes_here(trimmed) {
                // This line has the closing `>` — tag is fully open.
                state.multi_line_tag = None;
                state.write_continuation_indent(&mut out, tag_name);
                out.push_str(trimmed);
                out.push('\n');
                // Block-level tags open an indent level; inline/void do not.
                // Self-closing (`/>`) also never opens a level.
                if is_block && !trimmed.trim_end().ends_with("/>") {
                    state.level += 1;
                }
            } else {
                // Continuation attribute line (no `>` yet).
                state.write_continuation_indent(&mut out, tag_name);
                out.push_str(trimmed);
                out.push('\n');
            }
            continue;
        }

        // --- Multi-line Askama tag continuation ---
        // A tag such as `{% macro card(` spreads its arguments over several
        // lines; they are indented one level in, and the line carrying `%}`
        // returns to the level the tag opened at.
        if let Some((effect, is_match, open_level)) = state.multi_line_template {
            if trimmed.contains("%}") {
                state.multi_line_template = None;
                state.write_indent_at(&mut out, open_level);
                out.push_str(trimmed);
                out.push('\n');
                state.close_effect(effect, is_match);
            } else {
                state.write_indent_at(&mut out, open_level + 1);
                out.push_str(trimmed);
                out.push('\n');
            }
            continue;
        }

        // Inside a raw/verbatim block: emit the line exactly as written.
        // The body of `<pre>` / `<textarea>` is whitespace-significant, and
        // `<script>` / `<style>` hold code whose own indentation is not ours
        // to rewrite — so no leading whitespace is added or removed.
        if state.in_raw {
            if is_raw_block_close(trimmed) {
                state.raw_depth = state.raw_depth.saturating_sub(1);
                if state.raw_depth == 0 {
                    state.in_raw = false;
                    // Emit the closing line.  When the closing tag is the whole
                    // line (e.g. `</style>`) give it current-level indentation —
                    // it is markup, not content.  When it is embedded at the end
                    // of content (e.g. `}</style>`) the line stays verbatim, so
                    // that the last content line keeps its own whitespace.
                    //
                    // `</pre>` and `</textarea>` are never re-indented: the
                    // whitespace in front of them is *inside* the element and
                    // renders, so moving the tag would change the page.
                    let close_on_own_line = (is_raw_close_tag(trimmed)
                        && !is_ws_significant_close_tag(trimmed))
                        || trimmed.starts_with("{%");
                    if close_on_own_line {
                        state.write_indent(&mut out);
                        out.push_str(trimmed);
                    } else {
                        out.push_str(line);
                    }
                    out.push('\n');
                } else {
                    out.push_str(line);
                    out.push('\n');
                }
            } else {
                out.push_str(line);
                out.push('\n');
            }
            continue;
        }

        // Detect raw block opening
        if is_raw_block_open(trimmed) {
            state.in_raw = true;
            state.raw_depth = 1;
            state.write_indent(&mut out);
            out.push_str(trimmed);
            out.push('\n');
            continue;
        }

        // --- Classify the line ---

        // 1. HTML closing tag at start of line → unindent before printing
        if let Some(tag) = parse_html_close_tag(trimmed) {
            if is_indent_html_tag(tag) {
                state.level = state.level.saturating_sub(1);
                state.write_indent(&mut out);
                out.push_str(trimmed);
                out.push('\n');
                // If the same line also has an open tag (e.g. </td><td>), handle that
                continue;
            }
        }

        // 2. Template tag classification
        if let Some((kw, inner)) = parse_template_tag(trimmed) {
            if let Some(effect) = classify(kw, inner, &mut pairs) {
                let is_match = kw == "match";
                let open_level = state.open_effect(effect);
                state.write_indent_at(&mut out, open_level);
                out.push_str(trimmed);
                out.push('\n');

                // The tag's `%}` is on a later line — hold the effect until the
                // continuation lines have been emitted.
                if !trimmed.contains("%}") {
                    state.multi_line_template = Some((effect, is_match, open_level));
                } else {
                    state.close_effect(effect, is_match);
                }
                continue;
            }
        }

        // 6. HTML opening tag → check if it increases indent
        if let Some((open_tag, is_self_closing, has_close_on_same_line)) =
            parse_html_open_tag(trimmed)
        {
            if is_indent_html_tag(open_tag) && !is_self_closing {
                // If the closing tag is also on this same line (e.g. <td>val</td>),
                // don't change the indent level.
                if has_close_on_same_line {
                    state.write_indent(&mut out);
                    out.push_str(trimmed);
                    out.push('\n');
                    continue;
                }

                let formatted = maybe_format_attributes(trimmed, state.level, opts);
                state.write_indent(&mut out);
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
            state.write_indent(&mut out);
            out.push_str(trimmed);
            out.push('\n');
            state.multi_line_tag = Some((tag_name, is_block));
            continue;
        }

        // 7. Default: emit at current indent level with attribute formatting
        let formatted = maybe_format_attributes(trimmed, state.level, opts);
        state.write_indent(&mut out);
        out.push_str(&formatted);
        out.push('\n');
    }

    // Ensure single trailing newline
    let result = out.trim_end_matches('\n').to_string();
    result + "\n"
}

// ── Keyword classification ──────────────────────────────────────────────────

/// What an Askama tag does to the indent level.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Effect {
    /// `{% if %}`, `{% for %}`, `{% macro %}`, … — print here, indent the body.
    Indent,
    /// `{% endif %}`, `{% endfor %}`, … — unindent, then print.
    Unindent,
    /// `{% else %}`, `{% elif %}` — print one level out, body stays indented.
    UnindentLine,
    /// `{% when %}` — reset to the enclosing `{% match %}` + 1, indent the arm.
    Branch,
    /// `{% endwhen %}` — close one match arm, back to the `{% when %}` level.
    BranchInnerEnd,
    /// `{% endmatch %}` — back to the level the `{% match %}` was printed at.
    BranchEnd,
    /// `{% include %}`, `{% break %}`, `{% let x = 1 %}`, … — print here.
    NoChange,
}

const INDENT_KEYWORDS: &[&str] = &[
    "if", "for", "macro", "block", "filter", "with", "raw", "match",
];

const UNINDENT_KEYWORDS: &[&str] = &[
    "endif",
    "endfor",
    "endmacro",
    "endblock",
    "endfilter",
    "endwith",
    "endraw",
    "endcall",
    "endlet",
    "endset",
];

const UNINDENT_LINE_KEYWORDS: &[&str] = &["else", "else if", "elif"];

const BRANCH_KEYWORDS: &[&str] = &["when"];

const BRANCH_END_KEYWORDS: &[&str] = &["endmatch"];

const NO_CHANGE_KEYWORDS: &[&str] = &[
    "import", "include", "extends", "break", "continue", "mut", "decl", "declare",
];

/// Classify a template tag line.  `inner` is the tag text without delimiters,
/// needed to tell `{% let x = 1 %}` (a statement) from `{% let x %}` (a block).
fn classify(kw: &str, inner: &str, pairs: &mut BlockPairs) -> Option<Effect> {
    if BRANCH_KEYWORDS.contains(&kw) {
        return Some(Effect::Branch);
    }
    if kw == "endwhen" {
        return Some(Effect::BranchInnerEnd);
    }
    if BRANCH_END_KEYWORDS.contains(&kw) {
        return Some(Effect::BranchEnd);
    }
    if UNINDENT_KEYWORDS.contains(&kw) {
        return Some(Effect::Unindent);
    }
    if UNINDENT_LINE_KEYWORDS.contains(&kw) {
        return Some(Effect::UnindentLine);
    }
    if matches!(kw, "let" | "set") {
        let opens = crate::formatter::let_opens_block(inner) && pairs.claim(kw);
        return Some(if opens {
            Effect::Indent
        } else {
            Effect::NoChange
        });
    }
    if kw == "call" {
        return Some(if pairs.claim(kw) {
            Effect::Indent
        } else {
            Effect::NoChange
        });
    }
    if NO_CHANGE_KEYWORDS.contains(&kw) {
        return Some(Effect::NoChange);
    }
    if INDENT_KEYWORDS.contains(&kw) {
        return Some(Effect::Indent);
    }
    None
}

// ── Tag parsers ─────────────────────────────────────────────────────────────

/// If line starts with `</tag`, return `tag` (raw, not lowercased).
fn parse_html_close_tag(line: &str) -> Option<&str> {
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
    Some(&rest[..end])
}

/// Split a template tag line into `(keyword, inner)`, e.g.
/// `{% when Some with (x) %}` → `("when", "when Some with (x)")`.
/// Also handles `{%- when -%}` whitespace-stripped variants.
///
/// `inner` keeps the whole tag body (delimiters and whitespace-control markers
/// removed) because some keywords need it — `{% let x = 1 %}` and
/// `{% let x %}` differ only in what follows the keyword.
fn parse_template_tag(line: &str) -> Option<(&str, &str)> {
    let s = line.trim();
    if !s.starts_with("{%") {
        return None;
    }
    let inner = s[2..].trim_start_matches(['-', '+', '~', ' ', '\t']);
    // Trim the tail only when the tag actually closes on this line.
    let inner = match inner.find("%}") {
        Some(end) => inner[..end].trim_end_matches(['-', '+', '~', ' ', '\t']),
        None => inner,
    };
    let kw = crate::formatter::leading_keyword(inner);
    if kw.is_empty() {
        None
    } else {
        Some((kw, inner))
    }
}

/// Allocation-free check: does `text` contain `</tag>`?
fn contains_close_tag(text: &str, tag: &str) -> bool {
    let n = tag.len();
    if n + 3 > text.len() {
        return false;
    }
    text.as_bytes().windows(n + 3).any(|w| {
        w[0] == b'<'
            && w[1] == b'/'
            && w[n + 2] == b'>'
            && w[2..n + 2].eq_ignore_ascii_case(tag.as_bytes())
    })
}

/// Returns `(tag_name, is_self_closing, has_matching_close_on_same_line)`.
/// `tag_name` is a slice into `line` (raw, not lowercased).
fn parse_html_open_tag(line: &str) -> Option<(&str, bool, bool)> {
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
    let tag = &rest[..end]; // raw, not lowercased

    // Use the shared scanner — correctly skips {%...%} containing `>`.
    let close_pos = super::find_html_tag_close(s)?;

    // Self-closing: the byte immediately before `>` is `/`.
    let self_closing = close_pos > 0 && s.as_bytes()[close_pos - 1] == b'/';

    // Check if there's a matching close tag after the opening tag.
    let after_open = &s[close_pos + 1..];
    let has_close = contains_close_tag(after_open, tag);

    Some((tag, self_closing, has_close))
}

fn is_raw_block_open(line: &str) -> bool {
    let s = line.trim();
    // Only open a raw block if the closing tag is NOT also on this line.
    for tag in RAW_CONTENT_TAGS {
        if s.starts_with(&format!("<{}", tag)) && !s.contains(&format!("</{}>", tag)) {
            return true;
        }
    }
    if let Some((kw, _)) = parse_template_tag(s) {
        return kw == "raw";
    }
    false
}

/// Does the line start with the closing tag of a raw-content element?
fn is_raw_close_tag(line: &str) -> bool {
    RAW_CONTENT_TAGS
        .iter()
        .any(|tag| line.starts_with(&format!("</{}>", tag)))
}

/// Raw-content elements whose body is whitespace-significant.  Anything before
/// their closing tag is rendered text, so that tag must not be moved.
const WS_SIGNIFICANT_TAGS: &[&str] = &["pre", "textarea"];

fn is_ws_significant_close_tag(line: &str) -> bool {
    WS_SIGNIFICANT_TAGS
        .iter()
        .any(|tag| line.starts_with(&format!("</{}>", tag)))
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
    let tag = rest[..end].to_string();
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
    if RAW_CONTENT_TAGS
        .iter()
        .any(|tag| s.contains(&format!("</{}>", tag)))
    {
        return true;
    }
    if let Some((kw, _)) = parse_template_tag(s) {
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
