//! Link and image destinations a renderer may emit.

/// Schemes a rendered link or image may name. Any other scheme (`javascript:`,
/// `vbscript:`, `data:text/html`) can run script once a browser, or a Markdown or Djot
/// renderer downstream, turns the destination into an `href`.
const SAFE_LINK_SCHEMES: [&str; 5] = ["http", "https", "mailto", "tel", "ftp"];

/// `url` when it is safe as a rendered link destination: an allowed scheme, or a
/// scheme-relative, relative or fragment-only reference. `None` means the renderer keeps
/// the link text and drops the link.
pub(crate) fn safe_link_url(url: &str) -> Option<&str> {
    destination_allowed(url, false).then_some(url)
}

/// [`safe_link_url`] for an image source, which may also be a `data:image/` URI: an image
/// element does not run script from its source, and documents embed images that way.
pub(crate) fn safe_image_url(url: &str) -> Option<&str> {
    destination_allowed(url, true).then_some(url)
}

fn destination_allowed(url: &str, image: bool) -> bool {
    // Browsers strip leading C0 controls and spaces, and every tab and newline, before
    // they read the scheme. ~keep
    let normalized: String = url
        .trim_start_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let prefix = &normalized[..normalized.find(['/', '?', '#']).unwrap_or(normalized.len())];
    // A Markdown or Djot reader decodes character references and backslash escapes in a
    // destination, so `&#58;` or `\:` there can become the scheme separator. ~keep
    if prefix.contains(['&', '\\']) {
        return false;
    }
    let Some(colon) = prefix.find(':') else {
        return true;
    };
    let scheme = &prefix[..colon];
    SAFE_LINK_SCHEMES.iter().any(|safe| scheme.eq_ignore_ascii_case(safe))
        || (image
            && scheme.eq_ignore_ascii_case("data")
            && normalized[colon + 1..]
                .get(..6)
                .is_some_and(|media| media.eq_ignore_ascii_case("image/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_link_url_refuses_script_schemes_in_every_spelling() {
        for url in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            " \u{0}\u{1f}javascript:alert(1)",
            "java\tscr\nipt:alert(1)",
            "vbscript:msgbox(1)",
            "data:text/html,<script>alert(1)</script>",
            "data:image/png;base64,AAAA",
            "&#106;avascript:alert(1)",
            "javascript&colon;alert(1)",
            "javascript\\:alert(1)",
            "file:///etc/passwd",
        ] {
            assert_eq!(safe_link_url(url), None, "{url:?}");
        }
    }

    #[test]
    fn safe_link_url_keeps_allowed_and_relative_destinations_unchanged() {
        for url in [
            "https://example.com/a?b=1&c=2",
            "HTTPS://example.com",
            "http://example.com",
            "mailto:a@example.com",
            "tel:+1555",
            "ftp://example.com",
            "//example.com",
            "page.html",
            "dir/a:b",
            "?q=1&r=2",
            "#frag",
            "",
        ] {
            assert_eq!(safe_link_url(url), Some(url), "{url:?}");
        }
    }

    #[test]
    fn safe_image_url_also_accepts_data_image_sources() {
        assert_eq!(
            safe_image_url("data:image/png;base64,AAAA"),
            Some("data:image/png;base64,AAAA")
        );
        assert_eq!(
            safe_image_url(" DATA:Image/svg+xml,<svg/>"),
            Some(" DATA:Image/svg+xml,<svg/>")
        );
        assert_eq!(safe_image_url("data:text/html,x"), None);
        assert_eq!(safe_image_url("data:image"), None);
        assert_eq!(safe_image_url("javascript:alert(1)"), None);
        assert_eq!(safe_image_url("media/a.png"), Some("media/a.png"));
    }
}
