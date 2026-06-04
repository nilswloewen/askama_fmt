//! Detection of `{# askama_fmt: ... #}` directives.
//!
//! Three directives are supported, each carried in an Askama comment so the
//! template engine itself ignores them:
//!
//! * `{# askama_fmt: skip-file #}` — anywhere in the file, suppresses all
//!   formatting (the input is returned byte-for-byte).
//! * `{# askama_fmt: off #}` ... `{# askama_fmt: on #}` — the bytes between
//!   the two markers (inclusive) are left untouched while the rest of the
//!   file is formatted normally.
//!
//! Region handling works by substituting each off..on span with a placeholder
//! Askama comment before the formatting passes run, then restoring the
//! original text afterwards.  The placeholder is also `{# ... #}`, so every
//! existing pass already treats it as a raw span / inert content.

/// Sentinel used to stand in for an extracted off..on region.  Made
/// deliberately distinctive so the chance of colliding with user content is
/// negligible.
const PLACEHOLDER_PREFIX: &str = "{# __askama_fmt_internal_skip_";
const PLACEHOLDER_SUFFIX: &str = "__ #}";

#[derive(Debug, PartialEq, Eq)]
enum Directive {
    SkipFile,
    Off,
    On,
}

struct DirectiveSpan {
    start: usize,
    end: usize,
    kind: Directive,
}

/// True iff the input contains an `askama_fmt: skip-file` directive in an
/// Askama comment.
pub fn has_skip_file(input: &str) -> bool {
    scan_directives(input).any(|d| d.kind == Directive::SkipFile)
}

/// Extract every `off`..`on` region and replace it with a placeholder Askama
/// comment.  Returns the rewritten string and the list of placeholder/original
/// pairs.  Unmatched `off` (no later `on`) and stray `on` directives are left
/// in place.
pub fn extract_regions(input: &str) -> (String, Vec<(String, String)>) {
    let directives: Vec<DirectiveSpan> = scan_directives(input).collect();
    if directives.is_empty() {
        return (input.to_string(), Vec::new());
    }

    let mut regions: Vec<(String, String)> = Vec::new();
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;
    let mut i = 0usize;
    while i < directives.len() {
        let d = &directives[i];
        if d.kind != Directive::Off {
            i += 1;
            continue;
        }
        // Find the next On directive after this Off.
        let on_idx = directives[i + 1..]
            .iter()
            .position(|x| x.kind == Directive::On)
            .map(|p| p + i + 1);
        let Some(on_idx) = on_idx else {
            // No matching On — leave the rest of the input untouched.
            i += 1;
            continue;
        };
        let on_end = directives[on_idx].end;
        out.push_str(&input[cursor..d.start]);
        let placeholder = format!(
            "{}{}{}",
            PLACEHOLDER_PREFIX,
            regions.len(),
            PLACEHOLDER_SUFFIX
        );
        out.push_str(&placeholder);
        let original = input[d.start..on_end].to_string();
        regions.push((placeholder, original));
        cursor = on_end;
        i = on_idx + 1;
    }
    out.push_str(&input[cursor..]);
    (out, regions)
}

/// Reverse of [`extract_regions`]: substitute every placeholder back with the
/// original off..on text it stood in for.
pub fn restore_regions(formatted: &str, regions: &[(String, String)]) -> String {
    if regions.is_empty() {
        return formatted.to_string();
    }
    let mut out = formatted.to_string();
    for (placeholder, original) in regions {
        if let Some(pos) = out.find(placeholder.as_str()) {
            out.replace_range(pos..pos + placeholder.len(), original);
        }
    }
    out
}

fn scan_directives(input: &str) -> DirectiveIter<'_> {
    DirectiveIter { input, pos: 0 }
}

struct DirectiveIter<'a> {
    input: &'a str,
    pos: usize,
}

impl Iterator for DirectiveIter<'_> {
    type Item = DirectiveSpan;
    fn next(&mut self) -> Option<DirectiveSpan> {
        let bytes = self.input.as_bytes();
        while self.pos + 1 < bytes.len() {
            if bytes[self.pos] == b'{' && bytes[self.pos + 1] == b'#' {
                let start = self.pos;
                let after = start + 2;
                match self.input[after..].find("#}") {
                    Some(rel) => {
                        let inner = &self.input[after..after + rel];
                        let end = after + rel + 2;
                        self.pos = end;
                        if let Some(kind) = parse_directive(inner) {
                            return Some(DirectiveSpan { start, end, kind });
                        }
                    }
                    None => {
                        self.pos = bytes.len();
                        return None;
                    }
                }
                continue;
            }
            self.pos += 1;
        }
        None
    }
}

fn parse_directive(inner: &str) -> Option<Directive> {
    // Strip Askama whitespace-control marks (`-`, `+`, `~`) at the ends.
    let s = inner
        .trim()
        .trim_start_matches(['-', '+', '~'])
        .trim_start()
        .trim_end_matches(['-', '+', '~'])
        .trim();
    let rest = s
        .strip_prefix("askama_fmt")?
        .trim_start()
        .strip_prefix(':')?
        .trim();
    match rest {
        "off" => Some(Directive::Off),
        "on" => Some(Directive::On),
        "skip-file" => Some(Directive::SkipFile),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_off() {
        assert_eq!(parse_directive(" askama_fmt: off "), Some(Directive::Off));
    }

    #[test]
    fn parse_skip_file() {
        assert_eq!(
            parse_directive(" askama_fmt: skip-file "),
            Some(Directive::SkipFile)
        );
    }

    #[test]
    fn parse_with_dash_whitespace_control() {
        assert_eq!(parse_directive("- askama_fmt: off -"), Some(Directive::Off));
    }

    #[test]
    fn has_skip_file_basic() {
        assert!(has_skip_file("{# askama_fmt: skip-file #}\n<div></div>"));
    }

    #[test]
    fn extract_simple_region() {
        let input = "before\n{# askama_fmt: off #}\nraw\n{# askama_fmt: on #}\nafter\n";
        let (stripped, regions) = extract_regions(input);
        assert_eq!(regions.len(), 1);
        assert!(stripped.contains("__askama_fmt_internal_skip_0__"));
        let restored = restore_regions(&stripped, &regions);
        assert_eq!(restored, input);
    }
}
