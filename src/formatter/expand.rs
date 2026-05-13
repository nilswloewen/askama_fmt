/// Put block-level HTML and Askama template tags on their own lines.
use crate::config::FormatOptions;

/// HTML tags that cause line breaks (block-level + structural inline-blocks).
/// Inline elements like `<span>`, `<a>`, `<strong>` etc. are intentionally excluded.
pub const BLOCK_HTML_TAGS: &[&str] = &[
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
/// Built at runtime so we can include custom_blocks / ignore_blocks.
fn template_break_keywords(opts: &FormatOptions) -> Vec<String> {
    let mut kws: Vec<String> = vec![
        "if".into(),
        "else".into(),
        "else if".into(),
        "endif".into(),
        "for".into(),
        "endfor".into(),
        "macro".into(),
        "endmacro".into(),
        "block".into(),
        "endblock".into(),
        "filter".into(),
        "endfilter".into(),
        "with".into(),
        "endwith".into(),
        "raw".into(),
        "endraw".into(),
        "include".into(),
        "extends".into(),
        "import".into(),
    ];
    // custom_blocks contribute both open and end forms
    for b in &opts.custom_blocks {
        kws.push(b.clone());
        kws.push(format!("end{}", b));
    }
    // custom_blocks_unindent_line also need their own lines
    for b in &opts.custom_blocks_unindent_line {
        if !kws.contains(b) {
            kws.push(b.clone());
        }
    }
    // ignore_blocks: only add endXXX (they still need a line), but NOT the
    // opening keyword itself — that stays inline.
    for b in &opts.ignore_blocks {
        let end = format!("end{}", b);
        if !kws.contains(&end) {
            kws.push(end);
        }
        // Remove the opening keyword if it was added above (it stays inline)
        kws.retain(|k| k != b);
    }
    kws
}

/// Returns true if `pos` falls inside a raw/ignored span in `html`.
/// Ignored spans: `{# ... #}`, `<!-- ... -->`, `{% raw %}...{% endraw %}`,
/// `<pre>...</pre>`, `<script>...</script>`, `<style>...</style>`.
fn inside_raw_span(html: &str, pos: usize) -> bool {
    // Quick scan: find all raw regions and check containment.
    // We do this lazily with a simple state machine over bytes.
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    macro_rules! starts_with_at {
        ($needle:expr, $at:expr) => {
            bytes[$at..].starts_with($needle.as_bytes())
        };
    }

    while i < len {
        if i > pos {
            break;
        }
        // {# comment #}
        if starts_with_at!("{#", i) {
            let end = find_close(html, i + 2, "#}");
            let end = end.unwrap_or(len);
            if pos > i && pos < end + 2 {
                return true;
            }
            i = end + 2;
            continue;
        }
        // <!-- HTML comment -->
        if starts_with_at!("<!--", i) {
            let end = find_close(html, i + 4, "-->");
            let end = end.unwrap_or(len);
            if pos > i && pos < end + 3 {
                return true;
            }
            i = end + 3;
            continue;
        }
        // {% raw %} ... {% endraw %}
        if starts_with_at!("{%", i) {
            if let Some(tag_end) = find_close(html, i + 2, "%}") {
                let inner = html[i + 2..tag_end].trim();
                if inner == "raw" || inner.starts_with("raw ") || inner.starts_with("raw\t") {
                    let raw_end = html[tag_end + 2..]
                        .find("{% endraw %}")
                        .or_else(|| html[tag_end + 2..].find("{%endraw%}"))
                        .map(|o| tag_end + 2 + o);
                    let raw_end = raw_end.unwrap_or(len);
                    if pos > i && pos < raw_end {
                        return true;
                    }
                    i = raw_end;
                    continue;
                }
                i = tag_end + 2;
                continue;
            }
        }
        // <pre>, <script>, <style>
        for tag in &["pre", "script", "style"] {
            let open = format!("<{}", tag);
            if starts_with_at!(open.as_str(), i) {
                let close_tag = format!("</{}>", tag);
                let block_end = html[i..].find(close_tag.as_str()).map(|o| i + o);
                let block_end = block_end.unwrap_or(len);
                if pos > i && pos < block_end + close_tag.len() {
                    return true;
                }
                i = block_end + close_tag.len();
                // restart outer loop
                break;
            }
        }
        i += 1;
    }
    false
}

fn find_close(s: &str, from: usize, needle: &str) -> Option<usize> {
    s[from..].find(needle).map(|o| from + o)
}

pub fn expand(html: &str, opts: &FormatOptions) -> String {
    let html_tags: Vec<String> = BLOCK_HTML_TAGS.iter().map(|s| s.to_string()).collect();
    let tmpl_kws = template_break_keywords(opts);

    let mut out = html.to_string();

    // Pass 1: HTML block tags — break before and after
    out = break_html_tags(&out, &html_tags);

    // Pass 2: Template tags — break before and after
    out = break_template_tags(&out, &tmpl_kws);

    // Collapse runs of blank lines to at most one blank line
    collapse_blank_lines(&out)
}

/// Insert `\n` before and after HTML block tags when not already on own line.
fn break_html_tags(html: &str, tags: &[String]) -> String {
    let mut out = String::with_capacity(html.len() + 256);
    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();
    let mut i = 0usize;

    while i < len {
        // Check for `<` that starts an HTML block tag (open or close)
        if chars[i] == '<' {
            let rest: String = chars[i..].iter().collect();

            // Try to match a block HTML tag at this position
            if let Some((matched, tag_len)) = match_html_block_tag(&rest, tags) {
                let byte_pos: usize = chars[..i].iter().collect::<String>().len();
                let in_raw = inside_raw_span(html, byte_pos);

                if in_raw {
                    // Just emit the character as-is
                    out.push(chars[i]);
                    i += 1;
                    continue;
                }

                // Break before unless already at the start of a (possibly indented) line.
                // "Already on its own line" means every char since the last \n is whitespace.
                let already_own_line = out
                    .rfind('\n')
                    .map(|nl| out[nl + 1..].chars().all(char::is_whitespace))
                    .unwrap_or(out.is_empty());
                if !already_own_line {
                    // Remove trailing whitespace on the current line, then add newline
                    while out.ends_with([' ', '\t']) {
                        out.pop();
                    }
                    out.push('\n');
                }
                out.push_str(&matched);
                i += tag_len;

                // Break after (unless immediately followed by newline)
                if i < len && chars[i] != '\n' {
                    out.push('\n');
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Match an HTML block opening/closing tag at the start of `s`.
/// Returns (full match string, char length of match) or None.
fn match_html_block_tag(s: &str, tags: &[String]) -> Option<(String, usize)> {
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
    if !tags.contains(&name) {
        return None;
    }

    // Use the shared scanner that correctly skips {%...%} containing `>`.
    let close_byte = super::find_html_tag_close(s)?;
    let matched = &s[..close_byte + 1];
    let char_len = matched.chars().count();
    Some((matched.to_string(), char_len))
}

/// Insert `\n` before and after Askama template tags.
fn break_template_tags(html: &str, kws: &[String]) -> String {
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
            && !inside_raw_span(html, i)
        {
            in_html_open_tag = true;
            html_attr_quote = None;
            // Fall through to emit `<` below.
        }

        // Look for `{%` (both are ASCII, safe to check by byte)
        if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b'%' {
            if inside_raw_span(html, i) {
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

                let should_break = kws.iter().any(|k| {
                    keyword == *k
                        || keyword.starts_with(&format!("{} ", k))
                        || keyword.starts_with(&format!("{}\t", k))
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
fn extract_keyword(inner: &str) -> String {
    let trimmed =
        inner.trim_start_matches(|c: char| c == '-' || c == '+' || c == '~' || c.is_whitespace());
    // "else if" is a two-word keyword in Askama
    if trimmed.starts_with("else if") {
        return "else if".to_string();
    }
    trimmed.split_whitespace().next().unwrap_or("").to_string()
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
