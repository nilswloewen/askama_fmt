use askama_fmt::{format, FormatOptions};
use pretty_assertions::assert_eq;

fn opts() -> FormatOptions {
    FormatOptions {
        indent: 4,
        custom_blocks: vec!["match".into()],
        custom_blocks_unindent_line: vec!["when".into()],
        ignore_blocks: vec!["call".into()],
        ..Default::default()
    }
}

// ── match / when ─────────────────────────────────────────────────────────────

#[test]
fn match_basic() {
    let src =
        "{% match value %}{% when Some with (x) %}<p>{{ x }}</p>{% when None %}{% endmatch %}";
    let expected = "\
{% match value %}
{% when Some with (x) %}
    <p>{{ x }}</p>
{% when None %}
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn match_wildcard_when() {
    let src = "{% match value %}{% when 1 %}<p>one</p>{% when _ %}{% endmatch %}";
    let expected = "\
{% match value %}
{% when 1 %}
    <p>one</p>
{% when _ %}
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn match_when_multiple_alternatives() {
    let src = "{% match value %}{% when 1 | 4 | 86 %}<p>multi</p>{% when _ %}{% endmatch %}";
    let expected = "\
{% match value %}
{% when 1 | 4 | 86 %}
    <p>multi</p>
{% when _ %}
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn match_when_result_type() {
    let src = "{% match result %}{% when Ok with (val) %}<p>{{ val }}</p>{% when Err with (e) %}<p>error: {{ e }}</p>{% endmatch %}";
    let expected = "\
{% match result %}
{% when Ok with (val) %}
    <p>{{ val }}</p>
{% when Err with (e) %}
    <p>error: {{ e }}</p>
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn match_when_struct_variant() {
    let src =
        "{% match value %}{% when Some { field: x } %}<p>{{ x }}</p>{% when _ %}{% endmatch %}";
    let expected = "\
{% match value %}
{% when Some { field: x } %}
    <p>{{ x }}</p>
{% when _ %}
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn match_inside_for() {
    let src = "{% for item in items %}{% match item %}{% when Some with (x) %}<p>{{ x }}</p>{% when None %}{% endmatch %}{% endfor %}";
    let expected = "\
{% for item in items %}
    {% match item %}
    {% when Some with (x) %}
        <p>{{ x }}</p>
    {% when None %}
    {% endmatch %}
{% endfor %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn match_nested() {
    let src = "{% match outer %}{% when Some with (o) %}{% match inner %}{% when Some with (i) %}<p>{{ i }}</p>{% when None %}{% endmatch %}{% when None %}{% endmatch %}";
    let expected = "\
{% match outer %}
{% when Some with (o) %}
    {% match inner %}
    {% when Some with (i) %}
        <p>{{ i }}</p>
    {% when None %}
    {% endmatch %}
{% when None %}
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

// ── let / variable binding ───────────────────────────────────────────────────

#[test]
fn let_before_match() {
    let src = r#"{% let url = self.url.as_ref() %}{% match url %}{% when Some with (u) %}<a href="{{ u }}">link</a>{% when None %}{% endmatch %}"#;
    let expected = "{% let url = self.url.as_ref() %}\n{% match url %}\n{% when Some with (u) %}\n    <a href=\"{{ u }}\">link</a>\n{% when None %}\n{% endmatch %}\n";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn let_binding() {
    let src = "{% let x = some_expr %}<p>{{ x }}</p>";
    let expected = "{% let x = some_expr %}\n<p>{{ x }}</p>\n";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn let_mut_binding() {
    let src = "{% let mut count = 0 %}<p>{{ count }}</p>";
    let expected = "{% let mut count = 0 %}\n<p>{{ count }}</p>\n";
    assert_eq!(format(src, &opts()), expected);
}

// ── if let (Rust pattern matching in if) ─────────────────────────────────────

#[test]
fn if_let_pattern() {
    let src =
        "{% if let Some(x) = value %}\n    <p>{{ x }}</p>\n    <span>extra</span>\n{% endif %}";
    let expected = "\
{% if let Some(x) = value %}
    <p>{{ x }}</p>
    <span>extra</span>
{% endif %}
";
    assert_eq!(format(src, &opts()), expected);
}

// ── call (single-line macro invocation) ──────────────────────────────────────

#[test]
fn call_inline_no_drift() {
    let src = "<table><tr><td>{% call icons::icon() %}</td><th>Label</th><td>val</td></tr></table>";
    let expected = "\
<table>
    <tr>
        <td>{% call icons::icon() %}</td>
        <th>Label</th>
        <td>val</td>
    </tr>
</table>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn call_in_table_rows_no_drift() {
    let src = "<table>{% for row in rows %}<tr><td>{% call icons::icon() %}</td><th>{{ row.label }}</th><td>{{ row.value }}</td></tr>{% endfor %}</table>";
    let expected = "\
<table>
    {% for row in rows %}
        <tr>
            <td>{% call icons::icon() %}</td>
            <th>{{ row.label }}</th>
            <td>{{ row.value }}</td>
        </tr>
    {% endfor %}
</table>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn import_and_call() {
    let src = r#"{%- import "macros.html" as m -%}<div>{% call m::icon() %}</div>"#;
    let expected = "{%- import \"macros.html\" as m -%}\n<div>{% call m::icon() %}</div>\n";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn call_block_form() {
    let src = "{% call wrapper() %}<p>inner</p>{% endcall %}";
    let expected = "{% call wrapper() %}\n<p>inner</p>\n{% endcall %}\n";
    assert_eq!(format(src, &opts()), expected);
}

// ── macro definition ─────────────────────────────────────────────────────────

#[test]
fn macro_definition() {
    let src = "{% macro my_macro(arg) %}<p>{{ arg }}</p>{% endmacro %}";
    let expected = "\
{% macro my_macro(arg) %}
    <p>{{ arg }}</p>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

// ── filter block ─────────────────────────────────────────────────────────────

#[test]
fn filter_block() {
    let src = "{% filter lower %}<p>SOME TEXT</p>{% endfilter %}";
    let expected = "{% filter lower %}\n    <p>SOME TEXT</p>\n{% endfilter %}\n";
    assert_eq!(format(src, &opts()), expected);
}

// ── for loop ─────────────────────────────────────────────────────────────────

#[test]
fn for_loop_first_variable() {
    let src = r#"{% for item in items %}{% if loop.first %}<li class="first">{{ item }}</li>{% else %}<li>{{ item }}</li>{% endif %}{% endfor %}"#;
    let expected = "{% for item in items %}\n    {% if loop.first %}\n        <li class=\"first\">{{ item }}</li>\n    {% else %}\n        <li>{{ item }}</li>\n    {% endif %}\n{% endfor %}\n";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn for_enumerate_single_line() {
    let src = "{% for (idx, item) in items.iter().enumerate() %}<div>{{ idx }}: {{ item }}</div>{% endfor %}";
    let expected = "{% for (idx, item) in items.iter().enumerate() %}<div>{{ idx }}: {{ item }}</div>{% endfor %}\n";
    assert_eq!(format(src, &opts()), expected);
}

// ── else if ──────────────────────────────────────────────────────────────────

#[test]
fn else_if_chain() {
    let src = "{% if a %}<p>a</p>{% else if b %}<p>b</p>{% else %}<p>c</p>{% endif %}";
    let expected = "\
{% if a %}
    <p>a</p>
{% else if b %}
    <p>b</p>
{% else %}
    <p>c</p>
{% endif %}
";
    assert_eq!(format(src, &opts()), expected);
}

// ── Template conditions containing `>` (comparison operator) ────────────────
//
// `{% if x > 0 %}` is valid Askama.  Every HTML-close scanner previously
// stopped at the raw `>` character, which truncated the tag at the wrong
// position.  The shared `find_html_tag_close` helper now skips `{%...%}`.

#[test]
fn template_condition_with_gt_in_attribute() {
    // `{% if count > 0 %}` as a conditional attribute: the `>` inside the
    // template tag must NOT be treated as the HTML tag's closing bracket.
    let src = r#"<form action="/submit" method="POST" {% if count > 0 %} has-items {% endif %}><p>content</p></form>"#;
    let expected = "\
<form action=\"/submit\"
      method=\"POST\"
      {% if count > 0 %}
      has-items
      {% endif %}>
    <p>content</p>
</form>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn template_condition_with_gt_idempotent() {
    // Already-formatted form with `>` inside a conditional attribute must be
    // stable on a second pass (multi_line_tag path).
    let src = "\
<form action=\"/submit\"
      method=\"POST\"
      {% if count > 0 %}
      has-items
      {% endif %}>
    <p>content</p>
</form>
";
    assert_eq!(format(src, &opts()), src);
}

#[test]
fn template_condition_with_gt_in_body() {
    // `{% if count > 0 %}` as normal body content (NOT inside an HTML attr)
    // must be expanded and indented — then the condense pass collapses the
    // short block back to one line.  The key correctness check is that the `>`
    // inside the template tag does not truncate anything.
    let src = r#"<div>{% if count > 0 %}<p>{{ count }} items</p>{% endif %}</div>"#;
    let out = format(src, &opts());
    // Condensed form: short enough to fit on one line.
    assert!(
        out.contains("{% if count > 0 %}"),
        "if keyword missing: {}",
        out
    );
    assert!(out.contains("{% endif %}"), "endif missing: {}", out);
    assert!(
        out.contains("{{ count }} items"),
        "content missing: {}",
        out
    );
    // Idempotent
    assert_eq!(format(&out, &opts()), out);
}

#[test]
fn template_expression_with_gt_in_attribute() {
    // `{% if max > 100 %}` as a conditional HTML attribute alongside a regular
    // `{{ }}` expression.  The `>` inside `{%...%}` must not be mistaken for
    // the HTML tag's closing bracket.  The condense pass then collapses the
    // short conditional to one line within the attribute list.
    let src =
        r#"<input type="range" {% if max > 100 %} class="large" {% endif %} value="{{ val }}" />"#;
    let out = format(src, &opts());
    // value="{{ val }}" must appear AFTER the conditional block (not eaten
    // by a truncated tag scan).
    assert!(
        out.contains("value=\"{{ val }}\" />"),
        "value attr missing or misplaced:\n{}",
        out
    );
    assert!(
        out.contains("{% if max > 100 %}"),
        "if condition with > missing:\n{}",
        out
    );
    // Idempotent
    assert_eq!(format(&out, &opts()), out);
}

// ── Template tags in HTML opening-tag attribute position ─────────────────────
//
// Askama allows template tags inside an HTML opening tag's attribute list to
// conditionally inject attributes, e.g.:
//
//   <form method="POST" {% if flag %} extra-attr {% endif %}>
//
// These must NOT be broken out as block-level tags (they are attributes, not
// content), and the form's body must still be correctly indented.

#[test]
fn form_conditional_attr_stays_inline() {
    // Compact input — formatter must keep {% if %}/{% endif %} inline in attrs.
    let src = r#"<form action="/submit" method="POST" {% if extra %} data-extra="true" {% endif %}><input type="text" name="q"></form>"#;
    let expected = "\
<form action=\"/submit\"
      method=\"POST\"
      {% if extra %}
      data-extra=\"true\"
      {% endif %}>
    <input type=\"text\" name=\"q\">
</form>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn form_conditional_attr_idempotent() {
    // Starting from the already-formatted output verifies the multi-line
    // open-tag indent path is stable on a second pass.
    let src = "\
<form action=\"/submit\"
      method=\"POST\"
      {% if extra %}
      data-extra=\"true\"
      {% endif %}>
    <input type=\"text\" name=\"q\">
</form>
";
    assert_eq!(format(src, &opts()), src);
}

// ── Self-closing tag attributes: no spurious `/` token ───────────────────────

#[test]
fn self_closing_attrs_no_slash_token() {
    // <input .../> with attrs longer than max_attribute_length.
    // The `/` before `>` must NOT appear as a separate attribute token.
    let src = r#"<input type="text" name="username" placeholder="Enter username" autocomplete="username" required />"#;
    let expected = "\
<input type=\"text\"
       name=\"username\"
       placeholder=\"Enter username\"
       autocomplete=\"username\"
       required />
";
    assert_eq!(format(src, &opts()), expected);
}

// ── Inline content after `>` must not inflate attrs_only_len ─────────────────

#[test]
fn inline_content_after_close_no_attr_break() {
    // `<a href="{{ url }}">{% call ... %} text</a>` — the template call and
    // trailing text come AFTER `>` and must not inflate attrs_only_len above
    // max_attribute_length, which would incorrectly break the short href attr.
    let src =
        r#"<p><a data-open-side-panel href="{{ create_url }}">{% call icons::add() %} Add</a></p>"#;
    let expected = "\
<p>
    <a data-open-side-panel href=\"{{ create_url }}\">{% call icons::add() %} Add</a>
</p>
";
    assert_eq!(format(src, &opts()), expected);
}

// ── Multi-byte characters in attributes (em dash, etc.) ──────────────────────

#[test]
fn multibyte_attr_value_not_truncated() {
    // data-testid contains an em dash (—, 3 UTF-8 bytes).
    // parse_attributes previously used char indices as byte offsets, which
    // truncated attribute values containing multi-byte characters.
    // Verify the em dash survives unmodified through attribute line-breaking.
    let src = "<div class=\"validation-error\" data-testid=\"error\u{2014}title__validation-error\" hx-target=\"main\">{{ msg }}</div>";
    let out = format(src, &opts());
    // All three attributes must appear on their own lines (total attr length > 70)
    // and the em dash must be preserved intact — not truncated to "error>" or similar.
    assert!(
        out.contains("data-testid=\"error\u{2014}title__validation-error\""),
        "em dash in data-testid was corrupted:\n{}",
        out
    );
    assert!(
        out.contains("class=\"validation-error\""),
        "class attr missing:\n{}",
        out
    );
    assert!(
        out.contains("hx-target=\"main\""),
        "hx-target attr missing:\n{}",
        out
    );
    // Output must be stable (idempotent)
    assert_eq!(format(&out, &opts()), out);
}

// ── <style> raw blocks ───────────────────────────────────────────────────────

#[test]
fn style_block_short_condensed_to_one_line() {
    // A short <style> block should be collapsed to one line by the condense
    // step and must remain stable on a second pass.
    let src = "\
<html>
<head>
<style type=\"text/css\">
body { margin: 0; }
</style>
</head>
</html>
";
    let expected = "\
<html>
    <head>
        <style type=\"text/css\">body { margin: 0; }</style>
    </head>
</html>
";
    let out = format(src, &opts());
    assert_eq!(out, expected);
    // Idempotent on a second pass
    assert_eq!(format(&out, &opts()), expected);
}

#[test]
fn style_block_embedded_close_idempotent() {
    // When the condense step partially collapses a <style> block, the closing
    // `}</style>` ends up embedded at the end of a CSS content line rather than
    // on its own line.  is_raw_block_close must detect this via `contains` and
    // emit it as-is (not re-indented), keeping both formatter passes identical.
    let src = "\
<head>
    <style type=\"text/css\">.link a {
color: blue;
}</style>
</head>
";
    assert_eq!(format(src, &opts()), src);
}
