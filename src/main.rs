use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;
use rayon::prelude::*;
use similar::TextDiff;

use askama_fmt::config::{CliOverrides, FormatOptions};

#[derive(Parser, Debug)]
#[command(
    name = "askama_fmt",
    about = "Formatter for Askama HTML templates",
    version
)]
struct Cli {
    /// Files, directories, or glob patterns to format (omit when using --stdin-filepath)
    #[arg(required_unless_present = "stdin_filepath")]
    src: Vec<String>,

    /// Read from stdin, write formatted output to stdout.
    /// The PATH is used only for config discovery and error messages;
    /// the file is not read or written.
    #[arg(long, value_name = "PATH")]
    stdin_filepath: Option<PathBuf>,

    /// Check mode: exit 1 if any file would change, don't write
    #[arg(long)]
    check: bool,

    /// Print a unified diff for each file that would change, exit 1 if any
    /// file would change (implies no writes)
    #[arg(long)]
    diff: bool,

    /// Spaces per indentation level [default: 4]
    #[arg(long, value_name = "N")]
    indent: Option<usize>,

    /// Maximum line length [default: 120]
    #[arg(long, value_name = "N")]
    max_line_length: Option<usize>,

    /// Maximum attribute length before breaking [default: 70]
    #[arg(long, value_name = "N")]
    max_attribute_length: Option<usize>,

    /// Comma-separated list of custom indent blocks (e.g. "match")
    #[arg(long, value_name = "LIST")]
    custom_blocks: Option<String>,

    /// Comma-separated list of custom unindent-line blocks (e.g. "when")
    #[arg(long, value_name = "LIST")]
    custom_blocks_unindent_line: Option<String>,

    /// Comma-separated list of blocks to treat as single-line (e.g. "call")
    #[arg(long, value_name = "LIST")]
    ignore_blocks: Option<String>,

    /// Explicit path to askama_fmt.toml config file
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let overrides = CliOverrides {
        indent: cli.indent,
        max_line_length: cli.max_line_length,
        max_attribute_length: cli.max_attribute_length,
        custom_blocks: cli.custom_blocks,
        custom_blocks_unindent_line: cli.custom_blocks_unindent_line,
        ignore_blocks: cli.ignore_blocks,
        config: cli.config.clone(),
    };

    // ── Stdin mode ────────────────────────────────────────────────────────────
    // Read from stdin, write formatted output to stdout.  The filepath is used
    // only to discover the nearest askama_fmt.toml.
    if let Some(ref filepath) = cli.stdin_filepath {
        let opts = load_opts(filepath, &overrides, &cli.config);
        let mut input = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut input) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(2);
        }
        let formatted = askama_fmt::format(&input, &opts);
        if let Err(e) = io::stdout().write_all(formatted.as_bytes()) {
            eprintln!("Error writing stdout: {}", e);
            process::exit(2);
        }
        return;
    }

    // ── File mode ─────────────────────────────────────────────────────────────
    // Collect all .askama.html files, load their options, then format in parallel.
    let all_files: Vec<(PathBuf, FormatOptions)> = cli
        .src
        .iter()
        .flat_map(|s| expand_src(s))
        .map(|file| {
            let opts = load_opts(&file, &overrides, &cli.config);
            (file, opts)
        })
        .collect();

    // Process in parallel; produce (path, Err(msg) | Ok(None=unchanged |
    // Some((original, formatted)))).
    type FileResult = (PathBuf, Result<Option<(String, String)>, String>);
    let results: Vec<FileResult> = all_files
        .par_iter()
        .map(|(file, opts)| {
            let result = std::fs::read_to_string(file)
                .map_err(|e| format!("Error reading {}: {}", file.display(), e))
                .map(|original| {
                    let formatted = askama_fmt::format(&original, opts);
                    if formatted != original {
                        Some((original, formatted))
                    } else {
                        None
                    }
                });
            (file.clone(), result)
        })
        .collect();

    let no_write = cli.check || cli.diff;
    let mut any_changed = false;

    for (file, result) in results {
        match result {
            Err(msg) => eprintln!("{}", msg),
            Ok(None) => {}
            Ok(Some((original, formatted))) => {
                any_changed = true;
                if cli.diff {
                    print_diff(&file, &original, &formatted);
                } else if no_write {
                    println!("Would reformat: {}", file.display());
                } else {
                    match std::fs::write(&file, &formatted) {
                        Ok(()) => println!("Reformatted: {}", file.display()),
                        Err(e) => eprintln!("Error writing {}: {}", file.display(), e),
                    }
                }
            }
        }
    }

    if any_changed && no_write {
        process::exit(1);
    }
}

fn print_diff(path: &Path, original: &str, formatted: &str) {
    let diff = TextDiff::from_lines(original, formatted);
    print!(
        "{}",
        diff.unified_diff().context_radius(3).header(
            &format!("a/{}", path.display()),
            &format!("b/{}", path.display()),
        )
    );
}

fn load_opts(
    file: &Path,
    overrides: &CliOverrides,
    explicit_config: &Option<PathBuf>,
) -> FormatOptions {
    let base = if let Some(cfg_path) = explicit_config {
        FormatOptions::from_file(cfg_path).unwrap_or_default()
    } else {
        let dir = file.parent().unwrap_or(std::path::Path::new("."));
        FormatOptions::find_and_load(dir)
    };
    base.apply_overrides(overrides)
}

fn expand_src(src: &str) -> Vec<PathBuf> {
    if src.contains(['*', '?', '[']) {
        match glob::glob(src) {
            Ok(paths) => paths
                .filter_map(|r| r.ok())
                .flat_map(|p| collect_files(&p))
                .collect(),
            Err(e) => {
                eprintln!("Invalid glob pattern '{}': {}", src, e);
                vec![]
            }
        }
    } else {
        collect_files(&PathBuf::from(src))
    }
}

fn collect_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut files = Vec::new();
    if path.is_dir() {
        for entry in ignore::Walk::new(path).flatten() {
            let p = entry.path().to_path_buf();
            if p.is_file()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().ends_with(".askama.html"))
            {
                files.push(p);
            }
        }
    }
    files
}
