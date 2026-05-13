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
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct FormatOptions {
    pub indent: usize,
    pub max_line_length: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 4,
            max_line_length: 120,
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
        self
    }
}

/// CLI flag overrides applied on top of `askama_fmt.toml` values.
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub indent: Option<usize>,
    pub max_line_length: Option<usize>,
    pub config: Option<PathBuf>,
}
