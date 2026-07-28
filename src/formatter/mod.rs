pub mod compress;
pub mod condense;
pub mod expand;
pub mod indent;
pub mod skip;

use crate::config::FormatOptions;

/// Returns the byte offset of the `>` that closes an HTML opening tag,
/// correctly skipping over Askama template tags (`{%...%}`) that may contain
/// comparison operators (e.g. `{% if x > 0 %}`), template expressions
/// (`{{...}}`), and quoted attribute values (`"..."` / `'...'`).
///
/// This is the authoritative HTML-tag-close scanner used by every pass.
/// The previously used ad-hoc scanners broke when `>` appeared inside a
/// template condition in the attribute list.
pub(crate) fn find_html_tag_close(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_q: Option<u8> = None;

    while i < len {
        match in_q {
            Some(q) if bytes[i] == q => {
                in_q = None;
                i += 1;
            }
            Some(_) => {
                i += 1;
            }
            None => {
                if bytes[i] == b'"' || bytes[i] == b'\'' {
                    in_q = Some(bytes[i]);
                    i += 1;
                } else if bytes[i] == b'>' {
                    return Some(i);
                } else if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b'%' {
                    // Skip {%...%} — may contain `>` in conditions.
                    i += 2;
                    while i + 1 < len {
                        if bytes[i] == b'%' && bytes[i + 1] == b'}' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                } else if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b'{' {
                    // Skip {{...}} expressions.
                    i += 2;
                    while i + 1 < len {
                        if bytes[i] == b'}' && bytes[i + 1] == b'}' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
        }
    }
    None
}

/// Byte ranges of every Askama template construct: `{% ... %}`, `{{ ... }}`
/// and `{# ... #}`.
///
/// Their contents are Rust-ish source, not markup, so no pass may treat text
/// inside them as HTML.  Askama 0.16 macro definitions make this load-bearing:
/// `{% macro row(cells: Vec<Td>) %}` contains `<Td>`, which the HTML scanners
/// would otherwise happily mistake for a `<td>` tag and rewrite.
///
/// Ranges are half-open (`start <= pos < end`) and sorted by `start`.
pub(crate) fn template_spans(s: &str) -> Vec<(usize, usize)> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut spans = Vec::new();
    let mut i = 0usize;

    while i + 1 < len {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        let close: &[u8; 2] = match bytes[i + 1] {
            b'%' => b"%}",
            b'{' => b"}}",
            b'#' => b"#}",
            _ => {
                i += 1;
                continue;
            }
        };
        let mut j = i + 2;
        while j + 1 < len {
            if bytes[j] == close[0] && bytes[j + 1] == close[1] {
                break;
            }
            j += 1;
        }
        let end = if j + 1 < len { j + 2 } else { len };
        spans.push((i, end));
        i = end;
    }
    spans
}

/// Leading keyword of a template tag's inner text.
///
/// Stops at the first character that can't be part of a keyword, so the
/// caller-args form `{% call(item) each(items) %}` still yields `"call"`.
/// `else if` is returned whole — it is the one two-word keyword.
pub(crate) fn leading_keyword(inner: &str) -> &str {
    let trimmed =
        inner.trim_start_matches(|c: char| matches!(c, '-' | '+' | '~') || c.is_whitespace());
    if trimmed.starts_with("else if") {
        return "else if";
    }
    let end = trimmed
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(trimmed.len());
    &trimmed[..end]
}

/// First keyword of a whole `{% … %}` tag, delimiters included.
/// `"{%- endcall -%}"` → `Some("endcall")`.
pub(crate) fn tag_keyword(tag: &str) -> Option<&str> {
    let inner = tag.strip_prefix("{%")?;
    let inner = inner.strip_suffix("%}").unwrap_or(inner);
    let kw = leading_keyword(inner);
    (!kw.is_empty()).then_some(kw)
}

/// Does this `{% let %}` / `{% set %}` tag open a block?
///
/// Askama 0.16 gives both keywords two shapes: `{% let x = expr %}` binds a
/// value and stands alone, while `{% let x %} … {% endlet %}` captures the
/// block's rendered output into `x`.  The block form is exactly the one
/// without a value, i.e. without `=`.
///
/// `inner` is the tag's text with the `{%`/`%}` delimiters and any whitespace
/// control markers already stripped.
pub(crate) fn let_opens_block(inner: &str) -> bool {
    let inner = inner.trim();
    let rest = match inner.split_once(char::is_whitespace) {
        Some(("let" | "set", rest)) => rest,
        _ => return false,
    };
    !rest.contains('=')
}

/// O(log n) check: is `pos` inside any span produced by [`template_spans`]?
pub(crate) fn in_span(spans: &[(usize, usize)], pos: usize) -> bool {
    let idx = spans.partition_point(|&(start, _)| start <= pos);
    idx > 0 && spans[idx - 1].1 > pos
}

/// Openers whose block-ness depends on a matching closer being present.
///
/// Askama 0.16 turned `{% call %}` into a block that must be closed by
/// `{% endcall %}`, and gave `{% let %}` / `{% set %}` a block form
/// (`{% let x %}…{% endlet %}`).  Templates written against askama ≤ 0.15 use
/// bare `{% call foo() %}` statements and `{% let x %}` forward declarations,
/// which must not shift the indent level.  Pairing each opener with a later,
/// still-unclaimed closer tells the two apart without a version flag.
pub(crate) struct BlockPairs {
    endcall: usize,
    endlet: usize,
    endset: usize,
}

impl BlockPairs {
    pub(crate) fn scan(html: &str) -> Self {
        let mut pairs = Self {
            endcall: 0,
            endlet: 0,
            endset: 0,
        };
        for &(start, end) in template_spans(html).iter() {
            let tag = &html[start..end];
            if !tag.starts_with("{%") {
                continue;
            }
            match tag_keyword(tag) {
                Some("endcall") => pairs.endcall += 1,
                Some("endlet") => pairs.endlet += 1,
                Some("endset") => pairs.endset += 1,
                _ => {}
            }
        }
        pairs
    }

    /// Claim the closer for `kw`, if one is still outstanding.
    pub(crate) fn claim(&mut self, kw: &str) -> bool {
        let slot = match kw {
            "call" => &mut self.endcall,
            "let" => &mut self.endlet,
            "set" => &mut self.endset,
            _ => return false,
        };
        if *slot == 0 {
            return false;
        }
        *slot -= 1;
        true
    }
}

pub fn format(input: &str, opts: &FormatOptions) -> String {
    if input.is_empty() {
        return input.to_string();
    }

    // `{# askama_fmt: skip-file #}` short-circuits the whole pipeline.
    if skip::has_skip_file(input) {
        return input.to_string();
    }

    // Detect and preserve original line endings
    let crlf = input.contains("\r\n");

    // Normalise to LF
    let normalised = input.replace("\r\n", "\n").replace('\r', "\n");

    // `{# askama_fmt: off #}` .. `{# askama_fmt: on #}` regions are pulled out
    // and replaced with comment placeholders before formatting, then patched
    // back in afterwards so their bytes survive verbatim.
    let (stripped, regions) = skip::extract_regions(&normalised);

    let compressed = compress::compress(&stripped);
    let expanded = expand::expand(&compressed);
    let cleaned = condense::clean_whitespace(&expanded);
    let indented = indent::indent(&cleaned, opts);
    let condensed = condense::condense(&indented, opts);

    let restored = skip::restore_regions(&condensed, &regions);

    // Restore CRLF if original used it
    if crlf {
        restored.replace('\n', "\r\n")
    } else {
        restored
    }
}
