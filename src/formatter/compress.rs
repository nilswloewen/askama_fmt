/// Flatten multi-line HTML opening tags back to a single line.
///
/// `<div\n    class="foo"\n    id="bar">` → `<div class="foo" id="bar">`
use fancy_regex::Regex;
use std::sync::OnceLock;

static HTML_TAG_RE: OnceLock<Regex> = OnceLock::new();

fn html_tag_re() -> &'static Regex {
    HTML_TAG_RE.get_or_init(|| {
        // Matches an HTML opening or closing tag that may span multiple lines.
        // Groups:
        //   1 = opening bracket (<, </, <!)
        //   2 = tag name
        //   3 = attributes (may be empty, may span lines)
        //   4 = closing bracket (> or />)
        Regex::new(r"(?s)(</?(?:!(?!--))?)([^\s>!/\[]+)((?:\s[^>]*)?)(\s*/?>)").unwrap()
    })
}

pub fn compress(html: &str) -> String {
    // Normalise line endings first
    let html = html.replace("\r\n", "\n").replace('\r', "\n");
    let tmpl = super::template_spans(&html);

    html_tag_re()
        .replace_all(&html, |caps: &fancy_regex::Captures<'_>| {
            // Never touch text inside a template tag.  Generic type hints in an
            // Askama 0.16 macro definition (`{% macro f(v: Vec<Item>) %}`) look
            // exactly like an HTML tag to this regex, and rewriting them would
            // lower-case the type name.
            if super::in_span(&tmpl, caps.get(0).unwrap().start()) {
                return caps[0].to_string();
            }

            let bracket = &caps[1];
            let tag = caps[2].to_lowercase();
            let attrs_raw = &caps[3];
            let close = caps[4].trim_start();

            // Don't flatten tags that contain Askama block-level template tags
            // ({%...%}) in their attribute list — the expand / indent passes
            // need them to stay on separate lines.  Template *expressions*
            // ({{...}}) are fine to compress.
            if attrs_raw.contains("{%") {
                return caps[0].to_string();
            }

            // Don't flatten if the regex has incorrectly captured past the tag
            // boundary into a raw-content block (e.g. `<style>` followed by CSS
            // on the next line).  The closing tag being inside `attrs_raw` is the
            // tell.
            if super::expand::RAW_CONTENT_TAGS
                .iter()
                .any(|t| attrs_raw.contains(&format!("</{}", t)))
            {
                return caps[0].to_string();
            }

            // Flatten multi-line attributes: join non-empty lines with a single space
            let attrs: String = if attrs_raw.trim().is_empty() {
                String::new()
            } else {
                let joined = attrs_raw
                    .split('\n')
                    .flat_map(|l| l.split('\r'))
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!(" {}", joined)
            };

            // Self-closing tags get a space before `/>` for consistency
            let close_out = if close == "/>" {
                " />".to_string()
            } else {
                close.to_string()
            };

            format!("{}{}{}{}", bracket, tag, attrs, close_out)
        })
        .into_owned()
}
