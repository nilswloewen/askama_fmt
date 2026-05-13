pub mod config;
pub(crate) mod formatter;

#[doc(inline)]
pub use config::FormatOptions;

/// Format an Askama HTML template string.
pub fn format(input: &str, opts: &FormatOptions) -> String {
    formatter::format(input, opts)
}
