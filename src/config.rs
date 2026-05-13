use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Error returned when loading a config file fails.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "could not read config file: {e}"),
            Self::Parse(e) => write!(f, "could not parse config file: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Parse(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        Self::Parse(e)
    }
}

/// All formatting options. Deserializes directly from `askama_fmt.toml`.
///
/// Example `askama_fmt.toml`:
/// ```toml
/// indent = 4
/// max_line_length = 120
/// custom_blocks = ["match"]
/// custom_blocks_unindent_line = ["when"]
/// ignore_blocks = ["call"]
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FormatOptions {
    pub indent: usize,
    pub max_line_length: usize,
    pub max_attribute_length: usize,
    /// Tags that open an indented block, e.g. `["match"]`
    pub custom_blocks: Vec<String>,
    /// Tags rendered at indent-1 without changing level, e.g. `["when"]`
    pub custom_blocks_unindent_line: Vec<String>,
    /// Tags kept single-line (not expanded), e.g. `["call"]`
    pub ignore_blocks: Vec<String>,
    pub preserve_blank_lines: bool,
    pub max_blank_lines: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 4,
            max_line_length: 120,
            max_attribute_length: 70,
            custom_blocks: vec![],
            custom_blocks_unindent_line: vec![],
            ignore_blocks: vec![],
            preserve_blank_lines: false,
            max_blank_lines: 0,
        }
    }
}

impl FormatOptions {
    /// Load from an explicit `askama_fmt.toml` path.
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    /// Walk up from `start_dir` looking for `askama_fmt.toml`.
    pub fn find_and_load(start_dir: &Path) -> Self {
        let mut dir = start_dir.to_path_buf();
        loop {
            let candidate = dir.join("askama_fmt.toml");
            if candidate.exists() {
                if let Ok(opts) = Self::from_file(&candidate) {
                    return opts;
                }
            }
            if dir.join(".git").exists() || !dir.pop() {
                break;
            }
        }
        Self::default()
    }

    pub fn apply_overrides(mut self, ov: &CliOverrides) -> Self {
        if let Some(v) = ov.indent {
            self.indent = v;
        }
        if let Some(v) = ov.max_line_length {
            self.max_line_length = v;
        }
        if let Some(v) = ov.max_attribute_length {
            self.max_attribute_length = v;
        }
        if let Some(ref v) = ov.custom_blocks {
            self.custom_blocks = split_csv(v);
        }
        if let Some(ref v) = ov.custom_blocks_unindent_line {
            self.custom_blocks_unindent_line = split_csv(v);
        }
        if let Some(ref v) = ov.ignore_blocks {
            self.ignore_blocks = split_csv(v);
        }
        self
    }
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// CLI flag overrides applied on top of `askama_fmt.toml` values.
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub indent: Option<usize>,
    pub max_line_length: Option<usize>,
    pub max_attribute_length: Option<usize>,
    pub custom_blocks: Option<String>,
    pub custom_blocks_unindent_line: Option<String>,
    pub ignore_blocks: Option<String>,
    pub config: Option<PathBuf>,
}
