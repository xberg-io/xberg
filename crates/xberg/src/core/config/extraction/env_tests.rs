use super::*;
use crate::core::config::processing::EmbeddingModelType;
use crate::utils::test_support::{child_test_root, run_child_test};

const CHILD_CASE: &str = "XBERG_EXTRACTION_ENV_CHILD_CASE";

fn run_env_case(case: &str, variables: &[(&str, &str)]) {
    let module = module_path!().strip_prefix("xberg::").expect("crate module prefix");
    let name = format!("{module}::{case}::child");
    run_child_test(&name, CHILD_CASE, case, |command| {
        // Keep the native loader environment, but exclude all ambient extraction overrides. ~keep
        command
            .env_clear()
            .envs(
                std::env::vars_os()
                    .filter(|(key, _)| !key.to_string_lossy().to_ascii_uppercase().starts_with("XBERG_")),
            )
            .env(CHILD_CASE, case)
            .envs(variables.iter().copied());
    });
}

macro_rules! isolated_env_test {
    ($(#[$attr:meta])* fn $case:ident [$($var:literal => $value:literal),+ $(,)?] $body:block) => {
        $(#[$attr])*
        #[test]
        fn $case() {
            run_env_case(stringify!($case), &[$(($var, $value)),+]);
        }

        mod $case {
            use super::*;

            #[test]
            #[ignore = "run in an isolated subprocess"]
            fn child() {
                let Some(case) = child_test_root(CHILD_CASE) else {
                    return;
                };
                assert_eq!(case.as_os_str(), std::ffi::OsStr::new(stringify!($case)));
                $body
            }
        }
    };
}

isolated_env_test! {
    fn embedding_plugin_and_vlm_embedding_model_are_mutually_exclusive [
        "XBERG_EMBEDDING_PLUGIN_NAME" => "my-embedder",
        "XBERG_VLM_EMBEDDING_MODEL" => "openai/text-embedding-3-small",
    ] {
        let mut config = ExtractionConfig::default();
        let err = config
            .apply_env_overrides()
            .expect_err("should reject conflicting embedding env vars");
        assert!(
            matches!(err, XbergError::Validation { .. }),
            "expected Validation, got {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("mutually exclusive"), "message: {msg}");
    }
}

isolated_env_test! {
    fn empty_embedding_plugin_name_rejected [
        "XBERG_EMBEDDING_PLUGIN_NAME" => "",
    ] {
        let mut config = ExtractionConfig::default();
        let err = config
            .apply_env_overrides()
            .expect_err("should reject empty plugin name");
        assert!(
            matches!(err, XbergError::Validation { .. }),
            "expected Validation, got {err:?}"
        );
    }
}

isolated_env_test! {
    fn embedding_plugin_env_sets_chunking_embedding_to_plugin_variant [
        "XBERG_EMBEDDING_PLUGIN_NAME" => "my-embedder",
    ] {
        let mut config = ExtractionConfig::default();
        config
            .apply_env_overrides()
            .expect("should succeed with only plugin name set");
        let chunking = config.chunking.as_ref().expect("chunking should be created");
        let embedding = chunking.embedding.as_ref().expect("embedding should be set");
        match &embedding.model {
            EmbeddingModelType::Plugin { name } => {
                assert_eq!(name, "my-embedder");
            }
            other => panic!("expected Plugin variant, got {other:?}"),
        }
    }
}

isolated_env_test! {
    /// Regression test for issue #1339: `XBERG_LLM_API_KEY` / `XBERG_LLM_BASE_URL`
    /// must be forwarded onto an existing VLM OCR config so a custom `base_url` can
    /// resolve its credential from the environment.
    fn llm_cred_env_forwarded_to_existing_vlm_config [
        "XBERG_LLM_API_KEY" => "sk-test-key",
        "XBERG_LLM_BASE_URL" => "https://eu.api.openai.com/v1/",
    ] {
        let mut config = ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig {
                vlm_config: Some(crate::core::config::LlmConfig {
                    model: "gpt-4o-mini".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        config.apply_env_overrides().expect("env overrides should apply");
        let vlm = config.ocr.unwrap().vlm_config.unwrap();
        assert_eq!(vlm.api_key.as_deref(), Some("sk-test-key"));
        assert_eq!(vlm.base_url.as_deref(), Some("https://eu.api.openai.com/v1/"));
    }
}

isolated_env_test! {
    /// The forwarding must never fabricate a VLM config: a credential in the
    /// environment configures an OCR path that is already enabled, and never turns
    /// one on (issue #1339). The same rule now holds for structured extraction —
    /// see `llm_cred_env_does_not_enable_structured_extraction` (issue #1421).
    fn llm_cred_env_does_not_create_vlm_config [
        "XBERG_LLM_API_KEY" => "sk-test-key",
    ] {
        let mut config = ExtractionConfig::default();
        config.apply_env_overrides().expect("env overrides should apply");
        assert!(
            config.ocr.as_ref().and_then(|o| o.vlm_config.as_ref()).is_none(),
            "VLM config must not be auto-created from LLM cred env vars"
        );
    }
}

isolated_env_test! {
    /// Regression test for issue #1421. `config.structured_extraction.is_some()` is
    /// the *only* gate the pipeline consults (`core/pipeline/mod.rs`), so fabricating
    /// the config from a credential registered a post-processor that had neither a
    /// model nor a schema and failed on every single document with
    /// "model must not be empty". A key in the environment must leave the feature off.
    fn llm_cred_env_does_not_enable_structured_extraction [
        "XBERG_LLM_API_KEY" => "sk-test-key",
        "XBERG_LLM_BASE_URL" => "https://eu.api.openai.com/v1/",
    ] {
        let mut config = ExtractionConfig::default();
        config.apply_env_overrides().expect("env overrides should apply");
        assert!(
            config.structured_extraction.is_none(),
            "a credential and an endpoint must not enable structured extraction; \
             got {:?}",
            config.structured_extraction
        );
    }
}

isolated_env_test! {
    /// The other half of #1421: suppressing the fabrication must not stop the
    /// credential reaching a structured extraction the user actually asked for.
    fn llm_cred_env_applies_to_existing_structured_extraction [
        "XBERG_LLM_API_KEY" => "sk-test-key",
        "XBERG_LLM_BASE_URL" => "https://eu.api.openai.com/v1/",
    ] {
        let mut config = ExtractionConfig {
            structured_extraction: Some(crate::core::config::StructuredExtractionConfig {
                schema: serde_json::json!({"type": "object"}),
                schema_name: "invoice".to_string(),
                schema_description: None,
                strict: false,
                prompt: None,
                llm: crate::core::config::LlmConfig {
                    model: "openai/gpt-4o".to_string(),
                    ..Default::default()
                },
            }),
            ..Default::default()
        };
        config.apply_env_overrides().expect("env overrides should apply");
        let structured = config
            .structured_extraction
            .expect("structured extraction stays enabled");
        assert_eq!(structured.llm.api_key.as_deref(), Some("sk-test-key"));
        assert_eq!(
            structured.llm.base_url.as_deref(),
            Some("https://eu.api.openai.com/v1/")
        );
        assert_eq!(
            structured.llm.model, "openai/gpt-4o",
            "the configured model must survive"
        );
        assert_eq!(structured.schema_name, "invoice", "the configured schema must survive");
    }
}

isolated_env_test! {
    fn paddle_model_env_vars_populate_paddle_ocr_settings [
        "XBERG_OCR_MODEL_VERSION" => "pp-ocrv5",
        "XBERG_OCR_MODEL_TIER" => "server",
    ] {
        let mut config = ExtractionConfig::default();
        config
            .apply_env_overrides()
            .expect("paddle model env vars should apply");
        let paddle = config
            .ocr
            .as_ref()
            .and_then(|o| o.paddle_ocr_settings.as_ref())
            .expect("paddle_ocr_settings should be populated");
        assert_eq!(paddle.model_version, "pp-ocrv5");
        assert_eq!(paddle.model_tier, "server");
    }
}

isolated_env_test! {
    fn paddle_model_env_wins_over_existing_config_and_preserves_other_keys [
        "XBERG_OCR_MODEL_VERSION" => "pp-ocrv5",
    ] {
        let mut config = ExtractionConfig {
            ocr: Some(OcrConfig {
                paddle_ocr_config: Some(serde_json::json!({
                    "model_version": "pp-ocrv6",
                    "drop_score": 0.7,
                })),
                ..OcrConfig::default()
            }),
            ..ExtractionConfig::default()
        };
        config
            .apply_env_overrides()
            .expect("paddle model env override should apply");
        let paddle = config.ocr.as_ref().unwrap().paddle_ocr_settings.as_ref().unwrap();
        assert_eq!(paddle.model_version, "pp-ocrv5");
        assert_eq!(paddle.drop_score, 0.7);
        assert_eq!(
            paddle.model_tier,
            crate::paddle_ocr::PaddleOcrConfig::default().model_tier
        );
    }
}

isolated_env_test! {
    fn csv_delimiter_env_sets_csv_config [
        "XBERG_CSV_DELIMITER" => ";",
    ] {
        let mut config = ExtractionConfig::default();
        config.apply_env_overrides().expect("valid delimiter should apply");
        assert_eq!(config.csv.as_ref().unwrap().delimiter.as_deref(), Some(";"));
    }
}

isolated_env_test! {
    fn csv_delimiter_env_rejects_multi_byte_value [
        "XBERG_CSV_DELIMITER" => "::",
    ] {
        let mut config = ExtractionConfig::default();
        let err = config
            .apply_env_overrides()
            .expect_err("multi-byte delimiter should be rejected");
        assert!(matches!(err, XbergError::Validation { .. }));
    }
}

isolated_env_test! {
    fn csv_comment_prefixes_env_sets_csv_config [
        "XBERG_CSV_COMMENT_PREFIXES" => "#, //",
    ] {
        let mut config = ExtractionConfig::default();
        config
            .apply_env_overrides()
            .expect("valid comment prefixes should apply");
        assert_eq!(
            config.csv.as_ref().unwrap().comment_prefixes,
            vec!["#".to_string(), "//".to_string()]
        );
    }
}

isolated_env_test! {
    fn csv_comment_prefixes_env_rejects_empty_value [
        "XBERG_CSV_COMMENT_PREFIXES" => " , ",
    ] {
        let mut config = ExtractionConfig::default();
        let err = config
            .apply_env_overrides()
            .expect_err("blank comment prefixes should be rejected");
        assert!(matches!(err, XbergError::Validation { .. }));
    }
}

isolated_env_test! {
    fn tessdata_model_env_var_selects_the_model_set [
        "XBERG_OCR_TESSDATA_MODEL" => "best",
    ] {
        let mut config = ExtractionConfig::default();
        config
            .apply_env_overrides()
            .expect("a valid tessdata model should apply");
        assert_eq!(
            config.ocr.as_ref().map(|ocr| ocr.tessdata_model),
            Some(crate::core::config::TessdataModel::Best)
        );
    }
}

isolated_env_test! {
    fn tessdata_model_env_var_rejects_an_unknown_model_set [
        "XBERG_OCR_TESSDATA_MODEL" => "medium",
    ] {
        let mut config = ExtractionConfig::default();
        let err = config
            .apply_env_overrides()
            .expect_err("an unknown tessdata model must be rejected");
        assert!(err.to_string().contains("XBERG_OCR_TESSDATA_MODEL"), "{err}");
    }
}
