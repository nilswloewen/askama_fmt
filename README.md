# askama_fmt

A formatter for [Askama](https://github.com/askama-rs/askama) HTML templates.

Formats `.askama.html` template files using the same compress → expand → indent → condense pipeline as [djLint](https://djlint.com), with first-class support for Askama's Rust-specific template syntax: `{% match %}`/`{% when %}`, `{% call macro() %}`, `{% if let %}`, `{% let %}`, and `else if` chains.

## Install

```sh
cargo install askama_fmt
```

Or from source:

```sh
git clone https://github.com/nilswloewen/askama_fmt
cd askama_fmt
cargo install --path .
```

## Usage

```sh
# Format all .askama.html files in a directory (recursive)
askama_fmt templates/

# Format using a glob pattern
askama_fmt "src/**/*.askama.html"

# Check without writing (exits 1 if any file would change — useful in CI)
askama_fmt --check templates/

# Format a single file
askama_fmt src/templates/user.askama.html
```

Config is read from the nearest `askama_fmt.toml` found by walking up from the target file's directory. CLI flags override config file values.

## Config

Drop an `askama_fmt.toml` at your project root (or anywhere in the directory tree):

```toml
indent = 4
max_line_length = 120
max_attribute_length = 70

# Treat {% match %}...{% endmatch %} as an indented block
custom_blocks = ["match"]

# Treat {% when %} like {% else %}: rendered at the parent indent level
custom_blocks_unindent_line = ["when"]

# Keep {% call macro() %} single-line (Askama macros have no {% endcall %})
ignore_blocks = ["call"]
```

See [`askama_fmt.toml`](askama_fmt.toml) for the full default config with all options documented.

## Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `indent` | integer | `4` | Spaces per indentation level |
| `max_line_length` | integer | `120` | Lines longer than this are a candidate for collapsing/expanding |
| `max_attribute_length` | integer | `70` | HTML attributes longer than this are broken one-per-line |
| `custom_blocks` | string array | `[]` | Template tags that open an indented block (and `end*` closes it) |
| `custom_blocks_unindent_line` | string array | `[]` | Tags rendered at one indent level above their content (like `else`) |
| `ignore_blocks` | string array | `[]` | Tags kept inline — not expanded or indented |
| `preserve_blank_lines` | bool | `false` | Keep blank lines as-is instead of collapsing them |
| `max_blank_lines` | integer | `0` | Maximum consecutive blank lines to preserve (when `preserve_blank_lines` is true) |

## CLI reference

```
askama_fmt [OPTIONS] [SRC]...

Arguments:
  [SRC]...  Files, directories, or glob patterns to format (targets *.askama.html)

Options:
      --stdin-filepath <PATH>          Read from stdin, write to stdout (PATH used for config discovery only)
      --check                          Exit 1 if any file would change, don't write
      --diff                           Print a unified diff for each file that would change, exit 1 if any
      --indent <N>                     Spaces per indentation level
      --max-line-length <N>            Maximum line length
      --max-attribute-length <N>       Maximum attribute length before breaking
      --custom-blocks <LIST>           Comma-separated indent blocks (e.g. "match")
      --custom-blocks-unindent-line <LIST>  e.g. "when"
      --ignore-blocks <LIST>           e.g. "call"
      --config <PATH>                  Explicit path to askama_fmt.toml
  -h, --help
  -V, --version
```

## Editor integration

Any editor that supports external formatters via stdin/stdout works with `--stdin-filepath`:

```sh
# Neovim / conform.nvim
{
  "askama_fmt",
  args = { "--stdin-filepath", "$FILENAME" },
  stdin = true,
}

# Generic: pipe current buffer through the formatter
askama_fmt --stdin-filepath path/to/file.askama.html < file.askama.html
```

The file is not read or written — the path is used only to find the nearest `askama_fmt.toml`.

## CI example

```yaml
- name: Check template formatting
  run: askama_fmt --check "src/**/*.askama.html"

# Review what would change without failing the build
- name: Show formatting diff
  run: askama_fmt --diff "src/**/*.askama.html" || true
```

## License

MIT — see [LICENSE](LICENSE).

Inspired by [djLint](https://djlint.com).

## How it works

Five sequential passes over the source text:

1. **Compress** — flatten multi-line HTML opening tags to a single line
2. **Expand** — put each block-level HTML tag and Askama template tag on its own line
3. **Clean** — strip trailing whitespace and collapse excess blank lines
4. **Indent** — walk line-by-line, maintaining an indent level driven by opening/closing tags
5. **Condense** — collapse short tag pairs back to a single line if they fit within `max_line_length`
