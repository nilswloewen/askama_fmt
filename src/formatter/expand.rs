/// Put block-level HTML and Askama template tags on their own lines.
/// HTML tags that cause line breaks (block-level + structural inline-blocks).
/// Inline elements like `<strong>` etc. are intentionally excluded.
pub const BLOCK_HTML_TAGS: &[&str] = &[
    "a",
    "address",
    "article",
    "aside",
    "audio",
    "blockquote",
    "body",
    "button",
    "canvas",
    "caption",
    "col",
    "colgroup",
    "datalist",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "iframe",
    "label",
    "legend",
    "li",
    "link",
    "main",
    "map",
    "menu",
    "meta",
    "nav",
    "noscript",
    "ol",
    "optgroup",
    "option",
    "output",
    "p",
    "picture",
    "pre",
    "progress",
    "script",
    "section",
    "select",
    "source",
    "span",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
    "video",
];

/// Askama template keywords that get their own lines.
const TEMPLATE_BREAK_KEYWORDS: &[&str] = &[
    "if",
    "else",
    "else if",
    "endif",
    "for",
    "endfor",
    "macro",
    "endmacro",
    "block",
    "endblock",
    "filter",
    "endfilter",
    "with",
    "endwith",
    "raw",
    "endraw",
    "include",
    "extends",
    "import",
    "match",
    "endmatch",
    "when",
];

// ── Raw-span pre-computation ─────────────────────────────────────────────────

/// Scan `html` once and return all raw/verbatim byte ranges as sorted
/// `(start, end)` pairs (exclusive: a position `p` is inside span `(s, e)`
/// iff `s < p && p < e`, matching the original `inside_raw_span` semantics).
///
/// Raw spans: `{# ... #}`, `<!-- ... -->`, `{% raw %}...{% endraw %}`,
/// `<pre>...</pre>`, `<script>...</script>`, `<style>...</style>`.
fn compute_raw_spans(html: &str) -> Vec<(usize, usize)> {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    let mut spans: Vec<(usize, usize)> = Vec::new();

    while i < len {
        // {# comment #}
        if bytes[i..].starts_with(b"{#") {
            let end = find_close(html, i + 2, "#}").unwrap_or(len);
            spans.push((i, end + 2));
            i = end + 2;
            continue;
        }
        // <!-- HTML comment -->
        if bytes[i..].starts_with(b"<!--") {
            let end = find_close(html, i + 4, "-->").unwrap_or(len);
            spans.push((i, end + 3));
            i = end + 3;
            continue;
        }
        // {% raw %} ... {% endraw %}
        if bytes[i..].starts_with(b"{%") {
            if let Some(tag_end) = find_close(html, i + 2, "%}") {
                let inner = html[i + 2..tag_end].trim();
                if inner == "raw" || inner.starts_with("raw ") || inner.starts_with("raw\t") {
                    let raw_end = html[tag_end + 2..]
                        .find("{% endraw %}")
                        .or_else(|| html[tag_end + 2..].find("{%endraw%}"))
                        .map(|o| tag_end + 2 + o)
                        .unwrap_or(len);
                    spans.push((i, raw_end));
                    i = raw_end;
                    continue;
                }
                i = tag_end + 2;
                continue;
            }
        }
        // <pre>, <script>, <style>
        if bytes[i] == b'<' {
            let mut advanced = false;
            for tag in &["pre", "script", "style"] {
                let open = format!("<{}", tag);
                if html[i..].starts_with(&open) {
                    let close_tag = format!("</{}>", tag);
                    let block_end = html[i..]
                        .find(close_tag.as_str())
                        .map(|o| i + o)
                        .unwrap_or(len);
                    spans.push((i, (block_end + close_tag.len()).min(len)));
                    i = (block_end + close_tag.len()).min(len);
                    advanced = true;
                    break;
                }
            }
            if advanced {
                continue;
            }
        }
        i += 1;
    }
    spans
}

/// O(log n) check: is `pos` inside any pre-computed raw span?
fn is_in_raw_span(spans: &[(usize, usize)], pos: usize) -> bool {
    // spans are sorted by start; find the last one with start < pos
    let idx = spans.partition_point(|&(start, _)| start < pos);
    idx > 0 && spans[idx - 1].1 > pos
}

fn find_close(s: &str, from: usize, needle: &str) -> Option<usize> {
    s[from..].find(needle).map(|o| from + o)
}

pub fn expand(html: &str) -> String {
    // Pre-compute raw spans once per pass so neither break function needs to
    // re-scan from byte 0 on every tag match.
    let raw1 = compute_raw_spans(html);
    let out = break_html_tags(html, BLOCK_HTML_TAGS, &raw1);

    let raw2 = compute_raw_spans(&out);
    let out = break_template_tags(&out, TEMPLATE_BREAK_KEYWORDS, &raw2);

    // Collapse runs of blank lines to at most one blank line
    collapse_blank_lines(&out)
}

/// Insert `\n` before and after HTML block tags when not already on own line.
fn break_html_tags(html: &str, tags: &[&str], spans: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(html.len() + 256);
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0usize; // byte offset

    while i < len {
        // Check for `<` that starts an HTML block tag (open or close)
        if bytes[i] == b'<' {
            let rest = &html[i..]; // free slice — no allocation

            // Try to match a block HTML tag at this position
            if let Some((matched, byte_len)) = match_html_block_tag(rest, tags) {
                if is_in_raw_span(spans, i) {
                    // Inside a raw span — emit one char as-is
                    out.push(b'<' as char);
                    i += 1;
                    continue;
                }

                // Break before unless already at the start of a (possibly indented) line.
                let already_own_line = out
                    .rfind('\n')
                    .map(|nl| out[nl + 1..].chars().all(char::is_whitespace))
                    .unwrap_or(out.is_empty());
                if !already_own_line {
                    while out.ends_with([' ', '\t']) {
                        out.pop();
                    }
                    out.push('\n');
                }
                out.push_str(&matched);
                i += byte_len;

                // Break after (unless immediately followed by newline)
                if i < len && bytes[i] != b'\n' {
                    out.push('\n');
                }
                continue;
            }
        }
        // Emit the next character — ASCII fast path avoids chars().next() overhead
        if bytes[i].is_ascii() {
            out.push(bytes[i] as char);
            i += 1;
        } else {
            let ch = html[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Match an HTML block opening/closing tag at the start of `s`.
/// Returns `(full match string, byte length of match)` or `None`.
fn match_html_block_tag(s: &str, tags: &[&str]) -> Option<(String, usize)> {
    if !s.starts_with('<') {
        return None;
    }
    let rest = &s[1..];
    let (_closing, rest2) = if let Some(stripped) = rest.strip_prefix('/') {
        (true, stripped)
    } else {
        (false, rest)
    };

    let name_end = rest2
        .find(|c: char| !c.is_alphanumeric() && c != '-')
        .unwrap_or(rest2.len());
    if name_end == 0 {
        return None;
    }
    let name = rest2[..name_end].to_lowercase();
    if !tags.contains(&name.as_str()) {
        return None;
    }

    // Use the shared scanner that correctly skips {%...%} containing `>`.
    let close_byte = super::find_html_tag_close(s)?;
    let matched = &s[..close_byte + 1];
    Some((matched.to_string(), close_byte + 1))
}

/// Insert `\n` before and after Askama template tags.
fn break_template_tags(html: &str, kws: &[&str], spans: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(html.len() + 256);
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0usize; // byte offset into `html`
                        // Track when we're inside an HTML opening tag's attribute list
                        // (between `<tagname` and the closing `>`). Template tags in that
                        // position are conditional attributes and must not be broken out.
    let mut in_html_open_tag = false;
    let mut html_attr_quote: Option<u8> = None;

    while i < len {
        // --- HTML open-tag state machine ---
        if in_html_open_tag {
            let b = bytes[i];
            match html_attr_quote {
                Some(q) if b == q => {
                    // Closing quote of an attribute value.
                    html_attr_quote = None;
                    let ch = html[i..].chars().next().unwrap();
                    out.push(ch);
                    i += ch.len_utf8();
                    continue;
                }
                Some(_) => {
                    // Inside a quoted attribute value — emit verbatim.
                    let ch = html[i..].chars().next().unwrap();
                    out.push(ch);
                    i += ch.len_utf8();
                    continue;
                }
                None => {
                    if b == b'"' || b == b'\'' {
                        html_attr_quote = Some(b);
                    } else if b == b'>' {
                        // Closing `>` of the HTML opening tag.
                        in_html_open_tag = false;
                    } else if b == b'{' && i + 1 < len && bytes[i + 1] == b'%' {
                        // Template tag in attribute position — emit as-is, no line break.
                        if let Some(tag_end) = find_template_tag_end(html, i + 2) {
                            out.push_str(&html[i..tag_end]);
                            i = tag_end;
                            continue;
                        }
                    } else if b == b'{' && i + 1 < len && bytes[i + 1] == b'{' {
                        // Template expression in attribute position — emit as-is.
                        if let Some(end) = find_close(html, i + 2, "}}") {
                            out.push_str(&html[i..end + 2]);
                            i = end + 2;
                            continue;
                        }
                    }
                    // Quote char, `>`, or regular attribute text: emit and advance.
                    let ch = html[i..].chars().next().unwrap();
                    out.push(ch);
                    i += ch.len_utf8();
                    continue;
                }
            }
        }

        // Detect the start of an HTML opening tag: `<letter` (not `</`, `<!`, etc.).
        if bytes[i] == b'<'
            && i + 1 < len
            && bytes[i + 1].is_ascii_alphabetic()
            && !is_in_raw_span(spans, i)
        {
            in_html_open_tag = true;
            html_attr_quote = None;
            // Fall through to emit `<` below.
        }

        // Look for `{%` (both are ASCII, safe to check by byte)
        if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b'%' {
            if is_in_raw_span(spans, i) {
                // Emit the `{` character via its proper UTF-8 slice, advance by one char
                let ch = html[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
                continue;
            }

            // Find end of template tag `%}`
            if let Some(tag_end) = find_template_tag_end(html, i + 2) {
                let full_tag = &html[i..tag_end];
                let inner =
                    html[i + 2..tag_end - 2].trim_matches(|c| c == '-' || c == '+' || c == '~');
                let keyword = extract_keyword(inner);

                let should_break = kws.iter().any(|&k| {
                    keyword == k
                        || (keyword.len() > k.len()
                            && keyword.starts_with(k)
                            && matches!(keyword.as_bytes()[k.len()], b' ' | b'\t'))
                });

                if should_break {
                    let already_own_line = out
                        .rfind('\n')
                        .map(|nl| out[nl + 1..].chars().all(char::is_whitespace))
                        .unwrap_or(out.is_empty());
                    if !already_own_line {
                        while out.ends_with([' ', '\t']) {
                            out.pop();
                        }
                        if !out.ends_with('\n') {
                            out.push('\n');
                        }
                    } else {
                        while out.ends_with([' ', '\t']) {
                            out.pop();
                        }
                    }
                    out.push_str(full_tag);
                    i = tag_end;
                    if i < len && bytes[i] != b'\n' {
                        out.push('\n');
                    }
                    continue;
                }
            }
        }
        // Emit the next Unicode character (not just a single byte).
        let ch = html[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Find the closing `%}` of a template tag, starting search from `from`.
/// Handles nested `{{ }}` inside attribute values.
fn find_template_tag_end(html: &str, from: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = from;
    let mut in_quote: Option<u8> = None;

    while i < len {
        match in_quote {
            Some(q) if bytes[i] == q => {
                in_quote = None;
                i += 1;
            }
            Some(_) => {
                i += 1;
            }
            None => {
                if bytes[i] == b'"' || bytes[i] == b'\'' {
                    in_quote = Some(bytes[i]);
                    i += 1;
                } else if i + 1 < len && bytes[i] == b'%' && bytes[i + 1] == b'}' {
                    return Some(i + 2);
                } else {
                    i += 1;
                }
            }
        }
    }
    None
}

/// Extract the first keyword from a template tag's inner content.
/// `"- if let Some(x) = val "` → `"if let"`... we just need the first word.
fn extract_keyword(inner: &str) -> &str {
    let trimmed =
        inner.trim_start_matches(|c: char| c == '-' || c == '+' || c == '~' || c.is_whitespace());
    // "else if" is a two-word keyword in Askama
    if trimmed.starts_with("else if") {
        return "else if";
    }
    trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(['-', '+', '~'])
}

fn collapse_blank_lines(html: &str) -> String {
    // Collapse 3+ consecutive newlines to 2
    let mut result = String::with_capacity(html.len());
    let mut consecutive_newlines = 0u32;
    for ch in html.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                result.push(ch);
            }
        } else {
            consecutive_newlines = 0;
            result.push(ch);
        }
    }
    result
}
