pub mod compress;
pub mod condense;
pub mod expand;
pub mod indent;

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

pub fn format(input: &str, opts: &FormatOptions) -> String {
    if input.is_empty() {
        return input.to_string();
    }

    // Detect and preserve original line endings
    let crlf = input.contains("\r\n");

    // Normalise to LF
    let normalised = input.replace("\r\n", "\n").replace('\r', "\n");

    let compressed = compress::compress(&normalised);
    let expanded = expand::expand(&compressed);
    let cleaned = condense::clean_whitespace(&expanded);
    let indented = indent::indent(&cleaned, opts);
    let condensed = condense::condense(&indented, opts);

    // Restore CRLF if original used it
    if crlf {
        condensed.replace('\n', "\r\n")
    } else {
        condensed
    }
}
