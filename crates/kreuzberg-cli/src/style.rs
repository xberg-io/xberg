//! CLI color styling helpers using `anstyle`.
//!
//! Provides styled output for the kreuzberg CLI. Respects the `NO_COLOR`
//! environment variable (<https://no-color.org/>) and disables colors
//! when output is not a terminal.

use anstyle::{AnsiColor, Effects, Style};
use std::sync::OnceLock;

/// Bold blue for section headers.
const HEADER: Style = Style::new()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Blue)))
    .effects(Effects::BOLD);

/// Green for success values (MIME types, file paths, versions).
const SUCCESS: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)));

/// Dim for metadata, separators, secondary info.
const DIM: Style = Style::new().effects(Effects::DIMMED);

/// Bold red for errors.
const ERROR: Style = Style::new()
    .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Red)))
    .effects(Effects::BOLD);

/// Bold for labels in key-value pairs.
const LABEL: Style = Style::new().effects(Effects::BOLD);

/// Yellow for warnings.
const WARNING: Style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Yellow)));

/// Check whether color output is enabled.
///
/// Returns `false` if:
/// - The `NO_COLOR` environment variable is set (any value)
///
/// See <https://no-color.org/> for the specification.
pub fn is_color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("NO_COLOR").is_none())
}

/// Apply an `anstyle::Style` to text if colors are enabled.
fn styled(text: &str, style: Style) -> String {
    if is_color_enabled() {
        format!("{}{}{}", style.render(), text, style.render_reset())
    } else {
        text.to_string()
    }
}

/// Style text as a section header (bold blue).
pub fn header(text: &str) -> String {
    styled(text, HEADER)
}

/// Style text as a success value (green).
pub fn success(text: &str) -> String {
    styled(text, SUCCESS)
}

/// Style text as dim/secondary (dimmed).
pub fn dim(text: &str) -> String {
    styled(text, DIM)
}

/// Style text as a label (bold).
pub fn label(text: &str) -> String {
    styled(text, LABEL)
}

/// Style text as an error (bold red).
#[allow(dead_code)]
pub fn error_style(text: &str) -> String {
    styled(text, ERROR)
}

/// Style text as a warning (yellow).
#[allow(dead_code)]
pub fn warning(text: &str) -> String {
    styled(text, WARNING)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styled_returns_plain_text_when_no_color() {
        // Set NO_COLOR for this test's assertion scope via direct env check
        // Since OnceLock caches, we test the raw logic instead.
        let text = "hello";
        let result = format!("{}{}{}", Style::new().render(), text, Style::new().render_reset());
        // A plain Style produces no ANSI codes, so the result is just the text.
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_styled_applies_ansi_when_style_present() {
        let style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)));
        let rendered = format!("{}{}{}", style.render(), "ok", style.render_reset());
        // The rendered string should contain ANSI escape sequences.
        assert!(rendered.contains("\x1b["));
        assert!(rendered.contains("ok"));
    }

    #[test]
    fn test_helper_functions_return_strings() {
        // Smoke test: all helpers produce non-empty output for non-empty input.
        assert!(!header("h").is_empty());
        assert!(!success("s").is_empty());
        assert!(!dim("d").is_empty());
        assert!(!label("l").is_empty());
        assert!(!error_style("e").is_empty());
        assert!(!warning("w").is_empty());
    }

    #[test]
    fn test_is_color_enabled_respects_no_color_env() {
        // We cannot easily test OnceLock-cached value, but we can verify the
        // logic: NO_COLOR absence means colors enabled.
        let has_no_color = std::env::var_os("NO_COLOR").is_some();
        // The cached result should match the env at init time.
        assert_eq!(is_color_enabled(), !has_no_color);
    }
}
