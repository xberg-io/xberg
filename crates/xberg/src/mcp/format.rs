//! MCP response formatting and configuration helpers.
//!
//! This module provides utilities for formatting extraction results and building configurations.

use crate::ExtractionConfig;

/// Build extraction config from MCP parameters.
///
/// Merges the provided config JSON (if any) with the default config using JSON-level
/// merge semantics. Unspecified fields in the JSON preserve their values from the default config.
pub(super) fn build_config(
    default_config: &ExtractionConfig,
    config_json: Option<serde_json::Value>,
) -> Result<ExtractionConfig, String> {
    crate::core::config::request_security::resolve_caller_extraction_config(default_config, config_json.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_config_with_no_config() {
        let default_config = ExtractionConfig::default();

        let config = build_config(&default_config, None).unwrap();
        assert_eq!(config.use_cache, default_config.use_cache);
    }

    #[test]
    fn test_build_config_with_config_json() {
        let default_config = ExtractionConfig::default();
        let config_json = serde_json::json!({
            "use_cache": false
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();
        assert!(!config.use_cache);
    }

    #[test]
    fn test_build_config_with_invalid_config_json() {
        let default_config = ExtractionConfig::default();
        let config_json = serde_json::json!({
            "use_cache": "not_a_boolean"
        });

        let result = build_config(&default_config, Some(config_json));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to deserialize"));
    }

    #[test]
    fn test_build_config_preserves_default_config_settings() {
        let default_config = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        let config = build_config(&default_config, None).unwrap();

        assert!(!config.use_cache);
    }

    #[test]
    fn test_build_config_overrides_default_settings() {
        let default_config = ExtractionConfig {
            use_cache: true,
            ..Default::default()
        };

        let config_json = serde_json::json!({
            "use_cache": false
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();
        assert!(!config.use_cache);
    }

    #[test]
    fn test_build_config_merges_partial_config() {
        let default_config = ExtractionConfig {
            use_cache: false,
            enable_quality_processing: true,
            force_ocr: false,
            ..Default::default()
        };

        let config_json = serde_json::json!({
            "force_ocr": true
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();

        assert!(!config.use_cache, "use_cache should be preserved from default config");
        assert!(
            config.enable_quality_processing,
            "enable_quality_processing should be preserved"
        );
        assert!(config.force_ocr, "force_ocr should be overridden to true");
    }

    #[test]
    fn test_build_config_merges_nested_config() {
        let default_config = ExtractionConfig {
            use_cache: true,
            ..Default::default()
        };

        let config_json = serde_json::json!({
            "output_format": "markdown"
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();

        assert!(config.use_cache, "use_cache should be preserved from default config");
        assert_eq!(
            config.output_format,
            crate::core::config::formats::OutputFormat::Markdown,
            "output_format should be overridden to markdown"
        );
    }

    #[test]
    fn test_build_config_merges_with_custom_defaults() {
        let default_config = ExtractionConfig {
            use_cache: false,
            enable_quality_processing: true,
            force_ocr: false,
            ..Default::default()
        };

        let config_json = serde_json::json!({
            "force_ocr": true,
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();

        assert!(config.force_ocr, "force_ocr should be overridden to true");
        assert!(
            !config.use_cache,
            "use_cache should be preserved from default config (false)"
        );
        assert!(
            config.enable_quality_processing,
            "enable_quality_processing should be preserved (true)"
        );
    }

    #[test]
    fn test_build_config_merges_multiple_fields() {
        let default_config = ExtractionConfig {
            use_cache: true,
            enable_quality_processing: false,
            force_ocr: true,
            ..Default::default()
        };

        let config_json = serde_json::json!({
            "use_cache": false,
            "output_format": "markdown",
        });

        let config = build_config(&default_config, Some(config_json)).unwrap();

        assert!(!config.use_cache, "use_cache should be overridden to false");
        assert_eq!(
            config.output_format,
            crate::core::config::formats::OutputFormat::Markdown,
            "output_format should be overridden to markdown"
        );
        assert!(
            config.force_ocr,
            "force_ocr should be preserved from default config (true)"
        );
        assert!(
            !config.enable_quality_processing,
            "enable_quality_processing should be preserved (false)"
        );
    }

    #[test]
    fn test_build_config_boolean_override_to_default_value() {
        let base = ExtractionConfig {
            use_cache: false,
            ..Default::default()
        };

        let override_json = serde_json::json!({"use_cache": true});

        let merged = build_config(&base, Some(override_json)).unwrap();

        assert!(
            merged.use_cache,
            "Should use explicit override even if it matches default"
        );
    }

    #[test]
    fn should_reject_caller_llm_transport_config_before_merging_trusted_defaults() {
        let default_config = ExtractionConfig {
            ocr: Some(crate::OcrConfig {
                vlm_config: Some(crate::LlmConfig {
                    model: "trusted/model".to_string(),
                    base_url: Some("https://trusted.example".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let safe_override = serde_json::json!({"force_ocr": true});
        assert!(
            build_config(&default_config, Some(safe_override))
                .expect("trusted default transport config must remain valid")
                .force_ocr
        );

        let caller_override = serde_json::json!({
            "ocr": {"vlm_config": {"model": "caller/model", "headers": {"Authorization": "secret"}}}
        });
        assert_eq!(
            build_config(&default_config, Some(caller_override)).expect_err("caller transport config must be rejected"),
            "Caller extraction config may not set ocr.vlm_config.headers"
        );
    }

    #[test]
    #[cfg(feature = "url-config-types")]
    fn should_preserve_operator_crawl_egress_while_applying_caller_owned_options() {
        let mut default_config = ExtractionConfig::default();
        default_config.url.crawl.ssrf.max_redirects = 1;
        default_config.url.crawl.ssrf.scheme_allowlist = vec!["https".to_string()];
        default_config.url.crawl.proxy = Some(crawlberg::ProxyConfig {
            url: "http://operator-proxy.internal:8080".to_string(),
            username: None,
            password: None,
        });

        let caller_override = serde_json::json!({
            "url": {"crawl": {
                "max_depth": 7,
                "custom_headers": {"x-caller": "caller-value"}
            }}
        });
        let merged =
            build_config(&default_config, Some(caller_override)).expect("caller-owned crawl options must remain valid");

        assert_eq!(merged.url.crawl.ssrf.max_redirects, 1);
        assert_eq!(merged.url.crawl.ssrf.scheme_allowlist, vec!["https"]);
        assert_eq!(
            merged.url.crawl.proxy.as_ref().map(|proxy| proxy.url.as_str()),
            Some("http://operator-proxy.internal:8080")
        );
        assert_eq!(merged.url.crawl.max_depth, Some(7));
        assert_eq!(
            merged.url.crawl.custom_headers.get("x-caller").map(String::as_str),
            Some("caller-value")
        );
    }

    fn hardened_operator() -> ExtractionConfig {
        ExtractionConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_archive_size: 1024 * 1024,
                max_pages: Some(50),
                ..Default::default()
            }),
            max_archive_depth: 1,
            extraction_timeout_secs: Some(30),
            max_embedded_file_bytes: Some(1024),
            ..Default::default()
        }
    }

    #[test]
    fn should_reject_caller_css_file_in_mcp_config() {
        let caller_override = serde_json::json!({"html_output": {"css_file": "/proc/self/environ"}});

        assert_eq!(
            build_config(&ExtractionConfig::default(), Some(caller_override)).expect_err("css_file must be rejected"),
            "Caller extraction config may not set html_output.css_file"
        );
    }

    #[test]
    fn should_keep_operator_limits_when_caller_raises_them() {
        let caller_override = serde_json::json!({
            "security_limits": {"max_archive_size": 4_000_000_000_u64, "max_pages": 100_000},
            "max_archive_depth": 64,
            "extraction_timeout_secs": 86_400,
            "max_embedded_file_bytes": 4_000_000_000_u64
        });

        let config = build_config(&hardened_operator(), Some(caller_override)).expect("limit overrides parse");
        let limits = config.security_limits.expect("security limits stay set");

        assert_eq!(limits.max_archive_size, 1024 * 1024);
        assert_eq!(limits.max_pages, Some(50));
        assert_eq!(config.max_archive_depth, 1);
        assert_eq!(config.extraction_timeout_secs, Some(30));
        assert_eq!(config.max_embedded_file_bytes, Some(1024));
    }

    #[test]
    fn should_keep_operator_limits_when_caller_removes_them() {
        let caller_override = serde_json::json!({
            "security_limits": null,
            "extraction_timeout_secs": null,
            "max_embedded_file_bytes": null
        });

        let config = build_config(&hardened_operator(), Some(caller_override)).expect("null overrides parse");

        assert_eq!(
            config
                .security_limits
                .expect("operator limits survive")
                .max_archive_size,
            1024 * 1024
        );
        assert_eq!(config.extraction_timeout_secs, Some(30));
        assert_eq!(config.max_embedded_file_bytes, Some(1024));
    }

    #[test]
    fn should_apply_caller_limits_stricter_than_operator() {
        let caller_override = serde_json::json!({
            "security_limits": {"max_archive_size": 1024, "max_pages": 2},
            "max_archive_depth": 0,
            "extraction_timeout_secs": 5
        });

        let config = build_config(&hardened_operator(), Some(caller_override)).expect("limit overrides parse");
        let limits = config.security_limits.expect("security limits stay set");

        assert_eq!(limits.max_archive_size, 1024);
        assert_eq!(limits.max_pages, Some(2));
        assert_eq!(config.max_archive_depth, 0);
        assert_eq!(config.extraction_timeout_secs, Some(5));
    }

    #[test]
    fn should_bound_caller_limits_by_defaults_when_operator_sets_none() {
        let caller_override = serde_json::json!({"security_limits": {"max_archive_size": 4_000_000_000_u64}});

        let config = build_config(&ExtractionConfig::default(), Some(caller_override)).expect("limit override parses");

        assert_eq!(
            config.security_limits.expect("caller limits stay set").max_archive_size,
            crate::extractors::security::SecurityLimits::default().max_archive_size
        );
    }
}
