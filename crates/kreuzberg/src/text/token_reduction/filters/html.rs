use once_cell::sync::Lazy;
use regex::Regex;

/// Regular expression for matching HTML comments.
/// Matches the pattern `<!-- ... -->` for removing HTML comments from text.
static HTML_COMMENT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--.*?-->").expect("HTML comment regex pattern is valid and should compile"));

/// Removes HTML comments from the input text.
///
/// This function uses a regex to strip out all HTML comment blocks (`<!-- ... -->`).
///
/// # Arguments
/// * `text` - The input text that may contain HTML comments
///
/// # Returns
/// A new `String` with all HTML comments removed
pub fn remove_html_comments(text: &str) -> String {
    if HTML_COMMENT_REGEX.is_match(text) {
        HTML_COMMENT_REGEX.replace_all(text, "").into_owned()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_html_comments() {
        let input = "Text before <!-- comment --> text after";
        let result = remove_html_comments(input);

        assert!(!result.contains("<!-- comment -->"));
        assert!(result.contains("Text before"));
        assert!(result.contains("text after"));
    }

    #[test]
    fn test_no_html_comments() {
        let input = "Text without comments";
        let result = remove_html_comments(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_lazy_regex_initialization() {
        let _ = &*HTML_COMMENT_REGEX;
    }
}
