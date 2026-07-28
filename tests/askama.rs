use askama_fmt::{format, FormatOptions};
use pretty_assertions::assert_eq;

fn opts() -> FormatOptions {
    FormatOptions {
        indent: 4,
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

#[test]
fn match_idempotent() {
    let src = "\
{% match value %}
    {% when Some with (x) %}
        <p>{{ x }}</p>
    {% when None %}
{% endmatch %}
";
    assert_eq!(format(src, &opts()), src);
}

// ── let / variable binding ───────────────────────────────────────────────────

#[test]
fn let_before_match() {
    let src = r#"{% let url = self.url.as_ref() %}{% match url %}{% when Some with (u) %}<a href="{{ u }}">link</a>{% when None %}{% endmatch %}"#;
    let expected = "{% let url = self.url.as_ref() %}\n{% match url %}\n    {% when Some with (u) %}\n        <a href=\"{{ u }}\">link</a>\n    {% when None %}\n{% endmatch %}\n";
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
    // Short enough to collapse, exactly like `{% if %}` / `{% for %}` pairs.
    let src = "{% call wrapper() %}<p>inner</p>{% endcall %}";
    let expected = "{% call wrapper() %}<p>inner</p>{% endcall %}\n";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn call_block_indents_body() {
    let src = "{% call card() %}<div class=\"body\"><p>Some reasonably long paragraph of content here</p></div>{% endcall %}";
    let expected = "\
{% call card() %}
    <div class=\"body\">
        <p>Some reasonably long paragraph of content here</p>
    </div>
{% endcall %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// A bare `{% call %}` with no `{% endcall %}` anywhere is the askama ≤ 0.15
/// statement form — it must not open an indent level.
#[test]
fn call_statement_form_does_not_indent() {
    let src = "<div>\n{% call icons::add() %}\n<p>after</p>\n</div>";
    let expected = "\
<div>
    {% call icons::add() %}
    <p>after</p>
</div>
";
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

#[test]
fn template_condition_with_gt_in_attribute() {
    let src = r#"<form action="/submit" method="POST" {% if count > 0 %} has-items {% endif %}><p>content</p></form>"#;
    let expected = "\
<form action=\"/submit\" method=\"POST\" {% if count > 0 %} has-items {% endif %}>
    <p>content</p>
</form>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn template_condition_with_gt_idempotent() {
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
    let src = r#"<div>{% if count > 0 %}<p>{{ count }} items</p>{% endif %}</div>"#;
    let out = format(src, &opts());
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
    assert_eq!(format(&out, &opts()), out);
}

#[test]
fn template_expression_with_gt_in_attribute() {
    let src =
        r#"<input type="range" {% if max > 100 %} class="large" {% endif %} value="{{ val }}" />"#;
    let out = format(src, &opts());
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
    assert_eq!(format(&out, &opts()), out);
}

// ── Template tags in HTML opening-tag attribute position ─────────────────────

#[test]
fn form_conditional_attr_stays_inline() {
    let src = r#"<form action="/submit" method="POST" {% if extra %} data-extra="true" {% endif %}><input type="text" name="q"></form>"#;
    let expected = "\
<form action=\"/submit\" method=\"POST\" {% if extra %} data-extra=\"true\" {% endif %}>
    <input type=\"text\" name=\"q\">
</form>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn form_conditional_attr_idempotent() {
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

// ── Self-closing tag attributes ───────────────────────────────────────────────

#[test]
fn self_closing_attrs_no_slash_token() {
    // 98 chars — fits within max_line_length=120, so stays on one line
    let src = r#"<input type="text" name="username" placeholder="Enter username" autocomplete="username" required />"#;
    let expected = "<input type=\"text\" name=\"username\" placeholder=\"Enter username\" autocomplete=\"username\" required />\n";
    assert_eq!(format(src, &opts()), expected);
}

// ── Inline content after `>` ─────────────────────────────────────────────────

#[test]
fn inline_content_after_close_no_attr_break() {
    let src =
        r#"<p><a data-open-side-panel href="{{ create_url }}">{% call icons::add() %} Add</a></p>"#;
    let expected = "\
<p>
    <a data-open-side-panel href=\"{{ create_url }}\">{% call icons::add() %} Add</a>
</p>
";
    assert_eq!(format(src, &opts()), expected);
}

// ── Multi-byte characters in attributes ──────────────────────────────────────

#[test]
fn multibyte_attr_value_not_truncated() {
    let src = "<div class=\"validation-error\" data-testid=\"error\u{2014}title__validation-error\" hx-target=\"main\">{{ msg }}</div>";
    let out = format(src, &opts());
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
    assert_eq!(format(&out, &opts()), out);
}

// ── <style> raw blocks ───────────────────────────────────────────────────────

#[test]
fn style_block_short_condensed_to_one_line() {
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
    assert_eq!(format(&out, &opts()), expected);
}

#[test]
fn style_block_embedded_close_idempotent() {
    let src = "\
<head>
    <style type=\"text/css\">.link a {
color: blue;
}</style>
</head>
";
    assert_eq!(format(src, &opts()), src);
}

// ── {# comment #} indentation ────────────────────────────────────────────────

#[test]
fn comment_indented_inside_if() {
    let src = "{% if show %}{# heading #}<h1>Hi</h1>{% endif %}";
    let expected = "\
{% if show %}
    {# heading #}
    <h1>Hi</h1>
{% endif %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn comment_indented_inside_for() {
    let src = "{% for item in items %}{# render item #}<li>{{ item }}</li>{% endfor %}";
    let expected = "\
{% for item in items %}
    {# render item #}
    <li>{{ item }}</li>
{% endfor %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn comment_idempotent() {
    let src = "\
{% if show %}
    {# heading #}
    <h1>Hi</h1>
{% endif %}
";
    assert_eq!(format(src, &opts()), src);
}

// ── {% raw %} pass-through ────────────────────────────────────────────────────

#[test]
fn raw_block_passthrough() {
    let src = "{% raw %}{{ not_a_variable }}{% endraw %}";
    let expected = "\
{% raw %}
{{ not_a_variable }}
{% endraw %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn raw_block_multiline_passthrough() {
    let src = "\
{% raw %}
{{ not_a_variable }}
{# not_a_comment #}
{% endraw %}
";
    assert_eq!(format(src, &opts()), src);
}

#[test]
fn raw_block_inside_if() {
    let src = "{% if show %}{% raw %}{{ x }}{% endraw %}{% endif %}";
    let expected = "\
{% if show %}
    {% raw %}
{{ x }}
    {% endraw %}
{% endif %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn raw_block_not_broken_by_expand() {
    let src = "{% raw %}<div><p>{{ x }}</p></div>{% endraw %}";
    let out = format(src, &opts());
    assert!(
        out.contains("<div><p>{{ x }}</p></div>"),
        "raw block contents were modified:\n{}",
        out
    );
}

// ── anchor indentation ────────────────────────────────────────────────────────

#[test]
fn anchor_with_template_children_indented() {
    let src = "\
<a href=\"{{ get_wo_step_url }}\" data-no-htmx>
{% call icons::receipt_text() %}
<span>Onboarding</span>
</a>
";
    let expected = "\
<a href=\"{{ get_wo_step_url }}\" data-no-htmx>
    {% call icons::receipt_text() %}
    <span>Onboarding</span>
</a>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn anchor_short_collapses_to_one_line() {
    let src = "<a href=\"/home\">\n    Home\n</a>\n";
    let expected = "<a href=\"/home\">Home</a>\n";
    assert_eq!(format(src, &opts()), expected);
}

// ── Ignore directives ────────────────────────────────────────────────────────

#[test]
fn skip_file_directive_returns_input_verbatim() {
    let src = "\
{# askama_fmt: skip-file #}
<div>
   <span>kept exactly</span>
        <p>indentation untouched</p>
</div>
";
    assert_eq!(format(src, &opts()), src);
}

#[test]
fn skip_file_directive_works_with_whitespace_control() {
    let src = "\
{#- askama_fmt: skip-file -#}
<div>  raw  </div>
";
    assert_eq!(format(src, &opts()), src);
}

#[test]
fn skip_file_directive_anywhere_in_file_skips_whole_file() {
    let src = "\
<div>
<span>not formatted</span>
</div>
{# askama_fmt: skip-file #}
";
    assert_eq!(format(src, &opts()), src);
}

#[test]
fn region_off_on_preserves_inner_content_verbatim() {
    let src = "\
<p>before</p>
{# askama_fmt: off #}
   <span>   weird   spacing   </span>
       <p>kept as-is</p>
{# askama_fmt: on #}
<p>after</p>
";
    let expected = "\
<p>before</p>
{# askama_fmt: off #}
   <span>   weird   spacing   </span>
       <p>kept as-is</p>
{# askama_fmt: on #}
<p>after</p>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn region_off_on_with_whitespace_control_marks() {
    let src = "\
<div>before</div>
{#- askama_fmt: off -#}
   <span>untouched</span>
{#- askama_fmt: on -#}
<div>after</div>
";
    let formatted = format(src, &opts());
    assert!(formatted.contains("   <span>untouched</span>"));
    assert!(formatted.contains("{#- askama_fmt: off -#}"));
    assert!(formatted.contains("{#- askama_fmt: on -#}"));
}

#[test]
fn region_off_on_does_not_format_template_syntax_inside() {
    // Askama syntax inside an off region must not be re-indented even though
    // it would normally open/close blocks.
    let src = "\
<div>
{# askama_fmt: off #}
{% if cond %}
<span>x</span>
{% endif %}
{# askama_fmt: on #}
</div>
";
    let formatted = format(src, &opts());
    assert!(formatted.contains("{% if cond %}\n<span>x</span>\n{% endif %}"));
}

#[test]
fn multiple_off_on_regions_in_one_file() {
    let src = "\
<div>a</div>
{# askama_fmt: off #}
  one
{# askama_fmt: on #}
<div>b</div>
{# askama_fmt: off #}
  two
{# askama_fmt: on #}
<div>c</div>
";
    let formatted = format(src, &opts());
    assert!(formatted.contains("  one"));
    assert!(formatted.contains("  two"));
    assert!(formatted.contains("<div>a</div>"));
    assert!(formatted.contains("<div>b</div>"));
    assert!(formatted.contains("<div>c</div>"));
}

#[test]
fn off_without_matching_on_is_inert() {
    // No matching `on` — the directive is recognised but no extraction happens,
    // so the rest of the file is formatted normally.
    let src = "<div><span>x</span></div>\n{# askama_fmt: off #}\n";
    let formatted = format(src, &opts());
    assert!(formatted.contains("{# askama_fmt: off #}"));
    assert!(formatted.contains("<div>"));
}

// ── Askama 0.16 syntax ───────────────────────────────────────────────────────

#[test]
fn elif_branch() {
    let src =
        "{% if a %}<p>1</p>{% elif b %}<p>2</p>{% elif c %}<p>3</p>{% else %}<p>4</p>{% endif %}";
    let expected = "\
{% if a %}
    <p>1</p>
{% elif b %}
    <p>2</p>
{% elif c %}
    <p>3</p>
{% else %}
    <p>4</p>
{% endif %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn let_block_form_indents_body() {
    let src =
        "{% let body %}<article><h1>T</h1><p>Captured block content</p></article>{% endlet %}";
    let expected = "\
{% let body %}
    <article>
        <h1>T</h1>
        <p>Captured block content</p>
    </article>
{% endlet %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn set_block_form_indents_body() {
    let src = "{% set body %}<div><p>Captured block content here</p></div>{% endset %}";
    let expected = "\
{% set body %}
    <div>
        <p>Captured block content here</p>
    </div>
{% endset %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// `{% let x = 1 %}` is a statement, not a block — no `{% endlet %}`, no indent.
#[test]
fn let_value_form_is_a_statement() {
    let src = "<div>\n{% let n = 1 %}\n<p>{{ n }}</p>\n</div>";
    let expected = "\
<div>
    {% let n = 1 %}
    <p>{{ n }}</p>
</div>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn endwhen_closes_match_arm() {
    let src = "{% match v %}{% when Some(a) %}<p>{{ a }}</p>{% endwhen %}{% when None %}<p>none</p>{% endwhen %}{% endmatch %}";
    let expected = "\
{% match v %}
    {% when Some(a) %}
        <p>{{ a }}</p>
    {% endwhen %}
    {% when None %}
        <p>none</p>
    {% endwhen %}
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn match_else_arm() {
    let src = "{% match v %}{% when A %}<b>a</b>{% else %}<b>other</b>{% endmatch %}";
    let expected = "\
{% match v %}
    {% when A %}
        <b>a</b>
    {% else %}
        <b>other</b>
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn break_and_continue_do_not_indent() {
    let src = "{% for i in v %}{% if i.skip %}{% continue %}{% endif %}{% if i.stop %}{% break %}{% endif %}<li>{{ i }}</li>{% endfor %}";
    let expected = "\
{% for i in v %}
    {% if i.skip %}
        {% continue %}
    {% endif %}
    {% if i.stop %}
        {% break %}
    {% endif %}
    <li>{{ i }}</li>
{% endfor %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn declare_and_mut_statements() {
    let src = "<div>\n{% declare total %}\n{% decl mut n %}\n{% let mut i = 0 %}\n{% mut i += 1 %}\n</div>";
    let expected = "\
<div>
    {% declare total %}
    {% decl mut n %}
    {% let mut i = 0 %}
    {% mut i += 1 %}
</div>
";
    assert_eq!(format(src, &opts()), expected);
}

// ── macro type hints ─────────────────────────────────────────────────────────

/// Generic type hints contain `<`/`>` that must never be read as HTML tags —
/// `Vec<Item>` used to come back out as `Vec<item>`.
#[test]
fn macro_type_hints_preserved() {
    let src = "{% macro m(a: u32, b: Vec<Item>, c: HashMap<String, Vec<Tr>>) %}<div>{{ a }}</div>{% endmacro %}";
    let expected = "\
{% macro m(a: u32, b: Vec<Item>, c: HashMap<String, Vec<Tr>>) %}
    <div>{{ a }}</div>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// Type names that collide with HTML tag names are the sharp edge here.
#[test]
fn macro_type_hints_colliding_with_html_tag_names() {
    let src = "{% macro row(cell: Td, body: Option<Body>, opts: Vec<Option<Select>>) %}<tr>{{ cell }}</tr>{% endmacro %}";
    let expected = "\
{% macro row(cell: Td, body: Option<Body>, opts: Vec<Option<Select>>) %}
    <tr>{{ cell }}</tr>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn macro_type_hints_with_defaults_and_refs() {
    let src = "{% macro card(title: &str, n: u32 = 3, tags: &[&str] = empty) %}<p>{{ title }}</p>{% endmacro %}";
    let expected = "\
{% macro card(title: &str, n: u32 = 3, tags: &[&str] = empty) %}
    <p>{{ title }}</p>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// A macro signature broken over several lines keeps its shape: arguments one
/// level in, the closing `) %}` back at the tag's own level.
#[test]
fn macro_multiline_signature() {
    let src = "\
{% macro card(
title: &str,
items: Vec<Item>,
) %}
<div>{{ title }}</div>
{% endmacro %}";
    let expected = "\
{% macro card(
    title: &str,
    items: Vec<Item>,
) %}
    <div>{{ title }}</div>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

// ── nested complex macros ────────────────────────────────────────────────────

/// A macro whose body nests a loop, a `{% call %}` block, a match with typed
/// arms, and an if/elif/else chain — every construct askama 0.16 added or
/// changed, stacked.
#[test]
fn nested_macro_with_call_match_and_branches() {
    let src = "\
{% macro table(rows: Vec<Row>, caption: &str = \"\") %}
<table class=\"data\">
{% for row in rows %}
{% call ui::row(row.cells) %}
{% match row.kind %}
{% when RowKind::Header %}
<th scope=\"col\">{{ row.label }}</th>
{% endwhen %}
{% when RowKind::Data with (weight) %}
{% if weight > 10 %}
<td class=\"heavy\">{{ row.label }}</td>
{% elif weight > 0 %}
<td>{{ row.label }}</td>
{% else %}
<td class=\"empty\"></td>
{% endif %}
{% when _ %}
<td></td>
{% endmatch %}
{% endcall %}
{% endfor %}
</table>
{% endmacro %}";
    let expected = "\
{% macro table(rows: Vec<Row>, caption: &str = \"\") %}
    <table class=\"data\">
        {% for row in rows %}
            {% call ui::row(row.cells) %}
                {% match row.kind %}
                    {% when RowKind::Header %}
                        <th scope=\"col\">{{ row.label }}</th>
                    {% endwhen %}
                    {% when RowKind::Data with (weight) %}
                        {% if weight > 10 %}
                            <td class=\"heavy\">{{ row.label }}</td>
                        {% elif weight > 0 %}
                            <td>{{ row.label }}</td>
                        {% else %}
                            <td class=\"empty\"></td>
                        {% endif %}
                    {% when _ %}
                        <td></td>
                {% endmatch %}
            {% endcall %}
        {% endfor %}
    </table>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// Macro containing a block-form `{% let %}` that itself wraps a match.
#[test]
fn nested_macro_with_block_let_around_match() {
    let src = "\
{% macro badge(kind: Option<Kind>, labels: HashMap<String, Vec<Label>>) %}
{% let text %}
{% match kind %}
{% when Some(k) %}{{ k }}
{% when None %}unknown
{% endmatch %}
{% endlet %}
<span class=\"badge\">{{ text }}</span>
{% endmacro %}";
    let expected = "\
{% macro badge(kind: Option<Kind>, labels: HashMap<String, Vec<Label>>) %}
    {% let text %}
        {% match kind %}
            {% when Some(k) %}
                {{ k }}
            {% when None %}
                unknown
        {% endmatch %}
    {% endlet %}
    <span class=\"badge\">{{ text }}</span>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// Macros defined inside blocks, calling each other, with a caller-args
/// `{% call(item) … %}` block in the middle.
#[test]
fn nested_macros_calling_each_other() {
    let src = "\
{% macro list(items: Vec<Item>) %}
<ul>
{% call(item) ui::each(items) %}
{% macro cell(v: Td) %}
<li class=\"cell\">{{ v }}</li>
{% endmacro %}
{% call cell(item) %}
<span>{{ item.label }}</span>
{% endcall %}
{% endcall %}
</ul>
{% endmacro %}";
    let expected = "\
{% macro list(items: Vec<Item>) %}
    <ul>
        {% call(item) ui::each(items) %}
            {% macro cell(v: Td) %}
                <li class=\"cell\">{{ v }}</li>
            {% endmacro %}
            {% call cell(item) %}<span>{{ item.label }}</span>{% endcall %}
        {% endcall %}
    </ul>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// Deep nesting must be stable: formatting twice changes nothing.
#[test]
fn nested_macro_is_idempotent() {
    let src = "\
{% macro deep(rows: Vec<Row>) %}
{% for r in rows %}
{% match r %}
{% when Row::A(x) %}
{% call cell(x) %}
<td class=\"a\">{{ x }}</td>
{% endcall %}
{% when Row::B %}
{% filter upper %}
<td>b</td>
{% endfilter %}
{% when _ %}
{% endmatch %}
{% endfor %}
{% endmacro %}";
    let once = format(src, &opts());
    let twice = format(&once, &opts());
    assert_eq!(once, twice);
}

// ── constructs taken from real-world templates ───────────────────────────────
//
// Every case below is a shape observed in a production Askama codebase that
// the suite did not previously exercise.

/// Whitespace-control marks on block tags — by far the most common shape in
/// real templates, and the indenter must ignore the `-` when classifying.
#[test]
fn whitespace_control_on_block_tags() {
    let src = "{%- if show -%}<div class=\"x\">{%- for i in items -%}<span>{{- i -}}</span>{%- endfor -%}</div>{%- endif -%}";
    let expected = "\
{%- if show -%}
    <div class=\"x\">
        {%- for i in items -%}<span>{{- i -}}</span>{%- endfor -%}
    </div>
{%- endif -%}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn whitespace_control_on_else_if_chain() {
    let src = "{%- if a -%}<p>a</p>{%- else if b -%}<p>b</p>{%- else -%}<p>c</p>{%- endif -%}";
    let expected = "\
{%- if a -%}
    <p>a</p>
{%- else if b -%}
    <p>b</p>
{%- else -%}
    <p>c</p>
{%- endif -%}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn whitespace_control_on_match_arms() {
    let src = "{% match role %}{%- when Role::Admin -%}<p>admin</p>{%- when _ -%}{%- endmatch -%}";
    let expected = "\
{% match role %}
    {%- when Role::Admin -%}
        <p>admin</p>
    {%- when _ -%}
{%- endmatch -%}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn for_tuple_destructuring() {
    let src = "{%- for (key, value) in pairs -%}<li>{{- key -}}={{- value -}}</li>{%- endfor -%}";
    let expected =
        "{%- for (key, value) in pairs -%}<li>{{- key -}}={{- value -}}</li>{%- endfor -%}\n";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn when_nested_tuple_pattern() {
    let src =
        "{% match v %}{% when Some with ((a, b, c)) %}<p>{{ a }}</p>{% when None %}{% endmatch %}";
    let expected = "\
{% match v %}
    {% when Some with ((a, b, c)) %}
        <p>{{ a }}</p>
    {% when None %}
{% endmatch %}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn expression_filters_untouched() {
    let src = "<div>{{ body|safe }}{{ \"check\"|lucide }}{{ x|round(2) }}</div>";
    let expected = "<div>{{ body|safe }}{{ \"check\"|lucide }}{{ x|round(2) }}</div>\n";
    assert_eq!(format(src, &opts()), expected);
}

/// Empty `{% call %}{% endcall %}` pairs are the dominant icon idiom; they must
/// rejoin onto one line rather than sitting split across two.
#[test]
fn empty_call_pair_rejoins() {
    let src = "<a href=\"#\">{% call icons::add() %}{% endcall %}<span>Add</span></a>";
    let expected = "\
<a href=\"#\">
    {% call icons::add() %}{% endcall %}
    <span>Add</span>
</a>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn empty_call_pairs_in_if_branches() {
    let src = "{%- if unread -%}{% call icons::bell() %}{% endcall %}{%- else -%}{% call icons::quiet() %}{% endcall %}{%- endif -%}";
    let expected = "\
{%- if unread -%}
    {% call icons::bell() %}{% endcall %}
{%- else -%}
    {% call icons::quiet() %}{% endcall %}
{%- endif -%}
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn import_as_then_call() {
    let src = "{%- import \"icons.askama.html\" as icons -%}<nav>{% call icons::menu() %}{% endcall %}</nav>";
    let expected = "\
{%- import \"icons.askama.html\" as icons -%}
<nav>
    {% call icons::menu() %}{% endcall %}
</nav>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn include_statement() {
    let src = "<div>{% include \"partials/header.html\" %}<p>body</p></div>";
    let expected = "\
<div>
    {% include \"partials/header.html\" %}
    <p>body</p>
</div>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn macro_args_with_string_defaults() {
    let src = "{% macro field(label: str, icon: str = \"\", value: str = \"\") %}<td>{{ label }}</td>{% endmacro %}";
    let expected = "\
{% macro field(label: str, icon: str = \"\", value: str = \"\") %}
    <td>{{ label }}</td>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// A path type (`br_types::CsrfToken`) in an argument list — the `::` must not
/// be mistaken for anything, and the signature stays byte-identical.
#[test]
fn macro_args_with_path_type() {
    let src = "{% macro video(id: str, tok: br_types::CsrfToken, secs: i32) %}<div>{{ id }}</div>{% endmacro %}";
    let expected = "\
{% macro video(id: str, tok: br_types::CsrfToken, secs: i32) %}
    <div>{{ id }}</div>
{% endmacro %}
";
    assert_eq!(format(src, &opts()), expected);
}

/// `<svg>` and its children are treated as inline — the graphic stays on one
/// line instead of being exploded element by element.
#[test]
fn inline_svg_stays_on_one_line() {
    let src = "<button><svg viewBox=\"0 0 24 24\" fill=\"none\"><path d=\"M4 4h16\" stroke=\"currentColor\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/></svg><span>Go</span></button>";
    let expected = "\
<button>
    <svg viewBox=\"0 0 24 24\" fill=\"none\"><path d=\"M4 4h16\" stroke=\"currentColor\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/></svg>
    <span>Go</span>
</button>
";
    assert_eq!(format(src, &opts()), expected);
}

/// The shape real templates use: a template tag inside a textarea. The body is
/// raw content, so it is emitted exactly as written rather than indented.
#[test]
fn textarea_with_whitespace_control() {
    let src = "<textarea name=\"{{- name -}}\">{%- if let Some(s) = form.summary -%}{{- s -}}{%- endif -%}</textarea>";
    let expected = "\
<textarea name=\"{{- name -}}\">
{%- if let Some(s) = form.summary -%}{{- s -}}{%- endif -%}</textarea>
";
    assert_eq!(format(src, &opts()), expected);
}

// ── raw-content elements ─────────────────────────────────────────────────────
//
// `<pre>` and `<textarea>` bodies are whitespace-significant, and
// `<script>` / `<style>` hold code. None of them may be re-indented.
// HTML drops a single newline directly after a `<pre>` / `<textarea>` start
// tag, so gaining one there does not change what the page renders.

#[test]
fn textarea_literal_content_is_verbatim() {
    let src = "<form><textarea name=\"notes\" rows=\"4\">Line one\nLine two\n  indented line</textarea></form>";
    let expected = "\
<form>
    <textarea name=\"notes\" rows=\"4\">
Line one
Line two
  indented line</textarea>
</form>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn textarea_closing_tag_is_not_indented() {
    // Indenting `</textarea>` would push whitespace into the field's value.
    let src = "<form>\n<textarea name=\"n\">Line one\n  indented\n</textarea>\n</form>";
    let expected = "\
<form>
    <textarea name=\"n\">
Line one
  indented
</textarea>
</form>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn pre_keeps_its_indentation() {
    let src = "<div>\n<pre>\n  two\n    four\nflush\n</pre>\n</div>";
    let expected = "\
<div>
    <pre>
  two
    four
flush
</pre>
</div>
";
    assert_eq!(format(src, &opts()), expected);
}

#[test]
fn script_keeps_its_own_indentation() {
    let src = "<head>\n<script>\nfunction f() {\n    return 1;\n}\n</script>\n</head>";
    let out = format(src, &opts());
    assert!(
        out.contains("function f() {\n    return 1;\n}"),
        "script body was re-indented:\n{}",
        out
    );
}

#[test]
fn raw_content_elements_are_idempotent() {
    let src = "\
<div>
    <pre>
  two
    four
</pre>
    <textarea name=\"n\">a
  b</textarea>
    <style>.a {
    color: red;
}</style>
</div>
";
    let once = format(src, &opts());
    assert_eq!(once, format(&once, &opts()));
}
