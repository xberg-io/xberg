//! Logging helpers for the Kreuzberg CLI.
//!
//! Provides a [`build_env_filter`] function that layers default third-party
//! transport suppressions on top of whatever the caller or `RUST_LOG` specifies.
//! User-supplied per-target rules in `RUST_LOG` always win because
//! [`EnvFilter::add_directive`] does not override existing per-target directives.

use tracing_subscriber::EnvFilter;

/// Third-party crates that are noisy at their own default level.
///
/// These are added as *fallback* directives: if `RUST_LOG` or `level_override`
/// already contain a per-target rule for any of these crates it takes precedence,
/// so the user can still do `RUST_LOG=ureq=debug` to restore full transport logs.
const QUIET_DIRECTIVES: &[&str] = &[
    "ureq=warn",
    "ureq_proto=warn",
    "rustls=warn",
    "hyper_util=warn",
    "hf_hub=info",
    "tower_http=info",
];

/// Extract the target crate name from a directive string like `"ureq=warn"`.
///
/// Returns the part before `=`, or `None` if there is no `=`.
fn directive_target(directive: &str) -> Option<&str> {
    directive.split_once('=').map(|(target, _)| target)
}

/// Build an [`EnvFilter`] with third-party transport crates suppressed by default.
///
/// Precedence (highest first):
/// 1. Per-target directives already present in `RUST_LOG` (or `level_override`).
/// 2. The [`QUIET_DIRECTIVES`] suppressions added here.
/// 3. Root level: `level_override` → `RUST_LOG` → `"info"`.
///
/// Per-target directives that the user has already set are **not** overridden:
/// we skip adding a quiet directive when the base filter already contains a
/// rule for the same target crate. This is necessary because
/// [`EnvFilter::add_directive`] appends rather than guards — a later-added
/// per-target directive for the same crate takes precedence.
///
/// # Arguments
///
/// * `level_override` — explicit root-level string from a CLI flag (e.g. `"debug"`).
///   When `Some`, it replaces `RUST_LOG` entirely for the root level.
pub fn build_env_filter(level_override: Option<&str>) -> EnvFilter {
    // Use try_new on user input so a malformed --log-level falls back to info
    // instead of panicking the CLI.
    let base = level_override
        .and_then(|level| EnvFilter::try_new(level).ok())
        .or_else(|| EnvFilter::try_from_default_env().ok())
        .unwrap_or_else(|| EnvFilter::new("info"));

    // Snapshot the existing directive set so we can skip quiet directives
    // whose target the user has already configured explicitly.
    let existing_targets: std::collections::HashSet<String> = base
        .to_string()
        .split(',')
        .filter_map(|chunk| directive_target(chunk).map(|t| t.trim().to_string()))
        .collect();

    QUIET_DIRECTIVES
        .iter()
        .filter(|directive| {
            // Only add the quiet directive when no per-target rule for this
            // exact crate already exists. Word-boundary match via tokenized
            // target set avoids `hf_hub` colliding with `hf_hub_server`.
            directive_target(directive)
                .map(|target| !existing_targets.contains(target))
                .unwrap_or(true)
        })
        .fold(base, |filter, directive| {
            filter.add_directive(directive.parse().expect("built-in logging directive must be valid"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse the directive string from an EnvFilter for assertion-level checks.
    ///
    /// `EnvFilter::to_string()` returns a comma-separated representation of all
    /// directives. We use this as a stable, public inspection surface.
    fn filter_directives(filter: &EnvFilter) -> String {
        filter.to_string()
    }

    #[test]
    fn default_filter_suppresses_ureq() {
        // No env, no override → ureq and ureq_proto must be suppressed.
        let filter = build_env_filter(None);
        let directives = filter_directives(&filter);
        assert!(
            directives.contains("ureq=warn"),
            "ureq=warn must be present in default filter; got: {directives}"
        );
        assert!(
            directives.contains("ureq_proto=warn"),
            "ureq_proto=warn must be present in default filter; got: {directives}"
        );
        assert!(
            directives.contains("rustls=warn"),
            "rustls=warn must be present in default filter; got: {directives}"
        );
    }

    #[test]
    fn default_filter_keeps_kreuzberg_info() {
        // Root level info → kreuzberg has no suppression applied.
        let filter = build_env_filter(None);
        let directives = filter_directives(&filter);
        assert!(
            !directives.contains("kreuzberg=warn") && !directives.contains("kreuzberg=error"),
            "kreuzberg must not be suppressed in the default filter; got: {directives}"
        );
    }

    #[test]
    fn env_override_wins_for_third_party() {
        // Simulate RUST_LOG=ureq=debug by passing it as the level_override.
        // build_env_filter must detect the existing ureq= directive and skip the
        // ureq=warn suppression, so ureq=debug survives in the final filter.
        let filter = build_env_filter(Some("info,ureq=debug"));
        let directives = filter.to_string();
        assert!(
            directives.contains("ureq=debug"),
            "user-supplied ureq=debug must be preserved; got: {directives}"
        );
        assert!(
            !directives.contains("ureq=warn"),
            "ureq=warn suppression must not be added when user already set ureq=debug; got: {directives}"
        );
    }

    #[test]
    fn level_override_wins() {
        // CLI flag "debug" → root must be debug; suppression directives still present.
        let filter = build_env_filter(Some("debug"));
        let directives = filter_directives(&filter);
        assert!(
            directives.contains("debug"),
            "root debug level must appear in filter with --log-level debug; got: {directives}"
        );
        // Suppression for ureq must still be layered on top.
        assert!(
            directives.contains("ureq=warn"),
            "ureq=warn suppression must still be present even under --log-level debug; got: {directives}"
        );
    }

    #[test]
    fn tower_http_suppressed_at_default() {
        // No override → tower_http must be suppressed.
        let filter = build_env_filter(None);
        let directives = filter_directives(&filter);
        assert!(
            directives.contains("tower_http=info") || directives.contains("tower_http=warn"),
            "tower_http must be suppressed at default level; got: {directives}"
        );
    }

    #[test]
    fn all_quiet_directives_are_valid() {
        // Ensure every built-in directive parses without panic.
        for directive in super::QUIET_DIRECTIVES {
            directive
                .parse::<tracing_subscriber::filter::Directive>()
                .expect("built-in directive is invalid");
        }
    }

    #[test]
    fn no_level_override_uses_info_root() {
        // Without RUST_LOG set and no override, root should default to info.
        // The directive string must not open with debug or trace as the root level.
        let filter = build_env_filter(None);
        let directives = filter_directives(&filter);
        // Root "debug" or "trace" as the first token would mean root is debug/trace.
        let root_is_noisier_than_info = directives.starts_with("debug") || directives.starts_with("trace");
        assert!(
            !root_is_noisier_than_info,
            "default root level must not be debug/trace without RUST_LOG; got: {directives}"
        );
    }

    #[test]
    fn hf_hub_suppressed_at_default() {
        let filter = build_env_filter(None);
        let directives = filter_directives(&filter);
        assert!(
            directives.contains("hf_hub=info"),
            "hf_hub must be suppressed to info at default; got: {directives}"
        );
    }

    #[test]
    fn hyper_util_suppressed_at_default() {
        let filter = build_env_filter(None);
        let directives = filter_directives(&filter);
        assert!(
            directives.contains("hyper_util=warn"),
            "hyper_util must be suppressed to warn at default; got: {directives}"
        );
    }

    #[test]
    fn malformed_level_override_falls_back_to_info() {
        // Garbage CLI flag must NOT panic — try_new returns Err and we fall back
        // to RUST_LOG / info default.
        let filter = build_env_filter(Some(":::garbage"));
        let directives = filter_directives(&filter);
        // Quiet directives should still be layered, proving we recovered.
        assert!(
            directives.contains("ureq=warn"),
            "ureq=warn must still be present after malformed override; got: {directives}"
        );
    }

    #[test]
    fn similar_target_name_does_not_block_suppression() {
        // A user-supplied directive for `hf_hub_server` must NOT cause the
        // `hf_hub=info` suppression to be skipped (regression test for the
        // earlier substring-containment bug).
        let filter = build_env_filter(Some("info,hf_hub_server=debug"));
        let directives = filter.to_string();
        assert!(
            directives.contains("hf_hub_server=debug"),
            "user directive for hf_hub_server must survive; got: {directives}"
        );
        assert!(
            directives.contains("hf_hub=info"),
            "hf_hub=info suppression must still be applied; got: {directives}"
        );
    }
}
