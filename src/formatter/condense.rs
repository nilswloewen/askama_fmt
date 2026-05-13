/// Collapse short multi-line tag pairs back to a single line,
/// and clean up trailing whitespace.
use crate::config::FormatOptions;
use regex::Regex;
use std::sync::OnceLock;

// ── HTML short-tag collapse ──────────────────────────────────────────────────

/// HTML tags that can be put back on one line if short enough.
/// Mirrors djLint's `optional_single_line_html_tags` exactly.
const COLLAPSIBLE_HTML_TAGS: &str =
    "a|button|h1|h2|h3|h4|h5|h6|td|th|strong|small|em|icon|span|title|\
     link|path|label|div|li|script|style|head|body|p|select|article|\
     option|legend|summary|dt|figcaption|tr";

/// Template tags that can be collapsed to one line.
/// Mirrors djLint's optional_single_line_template_tags — excludes macro, filter, match.
const COLLAPSIBLE_TEMPLATE_TAGS: &str = "if|for|block|with";

static HTML_CONDENSE_RE: OnceLock<Regex> = OnceLock::new();
static TMPL_CONDENSE_RE: OnceLock<Regex> = OnceLock::new();

fn html_condense_re() -> &'static Regex {
    HTML_CONDENSE_RE.get_or_init(|| {
        // (<tag ...>)  \s*  ([^<]*)  \s*  (</tag>)
        // No lookahead needed — [^<] naturally prevents consuming nested tags.
        // (?si): s=dotall (not needed here but harmless), i=case-insensitive.
        let pat = format!(
            "(?si)(<(?:{t})\\b(?:[^>\"']*|\"[^\"]*\"|'[^']*')*>)\\s*([^<]*?)\\s*(</(?:{t})>)",
            t = COLLAPSIBLE_HTML_TAGS
        );
        Regex::new(&pat).unwrap()
    })
}

fn tmpl_condense_re() -> &'static Regex {
    TMPL_CONDENSE_RE.get_or_init(|| {
        // ({%[-+~]? tag [^%\n]* %})  \s*  ([^%\n]*)  \s*  ({%[-+~]? endtag ... %})
        // [^%\n]* — tag body and content must each be single-line.
        // Using standard regex crate (no lookahead needed here).
        let pat = format!(
            "(?im)(\\{{%-?[ ]*(?:{t})\\b[^%\\n]*%\\}})[ \\t]*\\n[ \\t]*([^%\\n]*)[ \\t]*\\n[ \\t]*(\\{{%-?[ ]*end(?:{t})\\b[^%\\n]*%\\}})",
            t = COLLAPSIBLE_TEMPLATE_TAGS
        );
        Regex::new(&pat).unwrap()
    })
}

pub fn condense(html: &str, opts: &FormatOptions) -> String {
    // First pass: collapse HTML tag pairs
    let html = html_condense_re()
        .replace_all(html, |caps: &regex::Captures<'_>| {
            let full_match = caps.get(0).unwrap().as_str();
            if !full_match.contains('\n') {
                return full_match.to_string();
            }
            let open = caps.get(1).unwrap().as_str().trim();
            let content = caps.get(2).unwrap().as_str().trim();
            let close = caps.get(3).unwrap().as_str().trim();
            let indent_len = leading_indent_of(html, caps.get(1).unwrap().start()).len();
            let combined = format!("{}{}{}", open, content, close);
            if combined.len() + indent_len <= opts.max_line_length {
                combined
            } else {
                full_match.to_string()
            }
        })
        .into_owned();

    // Second pass: collapse template tag pairs (now that HTML pairs are on one line)
    tmpl_condense_re()
        .replace_all(&html, |caps: &regex::Captures<'_>| {
            let full_match = caps.get(0).unwrap().as_str();
            if !full_match.contains('\n') {
                return full_match.to_string();
            }
            // Don't collapse a template pair whose closing tag is immediately
            // followed by `>` — that means it's inside an HTML opening tag's
            // attribute list (e.g. `{% if cond %} attr {% endif %}>`) and must
            // stay on separate lines so the HTML tag is parsed correctly.
            let match_end = caps.get(0).unwrap().end();
            let after_match = &html[match_end..];
            let next_non_ws = after_match.chars().find(|c| !c.is_whitespace());
            if next_non_ws == Some('>') {
                return full_match.to_string();
            }
            let open = caps.get(1).unwrap().as_str().trim();
            let content = caps.get(2).unwrap().as_str().trim();
            let close = caps.get(3).unwrap().as_str().trim();
            let indent_len = leading_indent_of(&html, caps.get(1).unwrap().start()).len();
            let combined = format!("{}{}{}", open, content, close);
            if combined.len() + indent_len <= opts.max_line_length {
                combined
            } else {
                full_match.to_string()
            }
        })
        .into_owned()
}

/// Strip trailing whitespace from each line, collapsing runs to at most one blank line.
pub fn clean_whitespace(html: &str, _opts: &FormatOptions) -> String {
    let mut result = String::with_capacity(html.len());
    let mut consecutive_blanks = 0u32;

    for line in html.lines() {
        let stripped = line.trim_end();
        if stripped.is_empty() {
            consecutive_blanks += 1;
            if consecutive_blanks <= 1 {
                result.push('\n');
            }
        } else {
            consecutive_blanks = 0;
            result.push_str(stripped);
            result.push('\n');
        }
    }
    result
}

/// Find the leading whitespace of the line that contains byte offset `pos`.
fn leading_indent_of(text: &str, pos: usize) -> String {
    let before = &text[..pos.min(text.len())];
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line = &text[line_start..];
    let spaces: String = line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    spaces
}
