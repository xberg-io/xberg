use super::super::*;
use xberg::ExtractionConfig;

use crate::commands::env_test;

const CHILD: &str = "commands::overrides::tests::llm_api_key::llm_api_key_env_child";
const API_KEY_ENV_VAR: &str = "XBERG_LLM_API_KEY";
const ENV_TEST_VALUE: &str = "env-value";

#[test]
fn cli_flag_takes_precedence_over_env_var() {
    env_test::run_case(CHILD, "cli-wins", API_KEY_ENV_VAR, Some(ENV_TEST_VALUE), true);
}

#[test]
fn env_var_used_when_cli_flag_absent() {
    env_test::run_case(CHILD, "env-fallback", API_KEY_ENV_VAR, Some(ENV_TEST_VALUE), true);
}

#[test]
fn returns_none_when_neither_source_is_set() {
    env_test::run_case(CHILD, "absent", API_KEY_ENV_VAR, None, true);
}

#[test]
fn empty_cli_flag_does_not_count() {
    env_test::run_case(CHILD, "empty-cli", API_KEY_ENV_VAR, Some(ENV_TEST_VALUE), true);
}

#[test]
fn llm_api_key_child_reports_a_wrong_expectation() {
    env_test::run_case(CHILD, "env-fallback", API_KEY_ENV_VAR, None, false);
}

#[test]
#[ignore = "run with an isolated environment by its parent test"]
fn llm_api_key_env_child() {
    let Ok(case) = std::env::var(env_test::CASE) else {
        return;
    };
    let (cli_value, expected) = match case.as_str() {
        "cli-wins" => (Some("cli-value"), Some("cli-value")),
        "env-fallback" => (None, Some(ENV_TEST_VALUE)),
        "absent" => (None, None),
        "empty-cli" => (Some("   "), Some(ENV_TEST_VALUE)),
        _ => panic!("unknown LLM API key test case"),
    };
    let actual = resolve_llm_api_key(cli_value);
    eprintln!("{}", env_test::OBSERVED);
    assert!(actual.as_deref() == expected, "{}", env_test::MISMATCH);
}

#[test]
fn apply_llm_api_key_fills_translation_slot() {
    use xberg::core::config::{LlmConfig, TranslationConfig};
    let mut config = ExtractionConfig {
        translation: Some(TranslationConfig {
            target_lang: "de".to_string(),
            source_lang: None,
            preserve_markup: false,
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                ..Default::default()
            },
        }),
        ..Default::default()
    };
    apply_llm_api_key(&mut config, "resolved");
    let t = config.translation.unwrap();
    assert_eq!(t.llm.api_key.as_deref(), Some("resolved"));
}

#[test]
fn apply_llm_api_key_preserves_existing_key() {
    use xberg::core::config::{LlmConfig, TranslationConfig};
    let mut config = ExtractionConfig {
        translation: Some(TranslationConfig {
            target_lang: "de".to_string(),
            source_lang: None,
            preserve_markup: false,
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                api_key: Some("explicit".to_string()),
                ..Default::default()
            },
        }),
        ..Default::default()
    };
    apply_llm_api_key(&mut config, "resolved");
    let t = config.translation.unwrap();
    assert_eq!(
        t.llm.api_key.as_deref(),
        Some("explicit"),
        "explicit config keys take precedence over the resolved value"
    );
}

#[test]
fn apply_llm_api_key_fills_page_classification_slot() {
    use xberg::core::config::{LlmConfig, PageClassificationConfig};
    let mut config = ExtractionConfig {
        page_classification: Some(PageClassificationConfig {
            prompt_template: None,
            labels: vec!["a".to_string()],
            multi_label: false,
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                ..Default::default()
            },
        }),
        ..Default::default()
    };
    apply_llm_api_key(&mut config, "resolved");
    let pc = config.page_classification.unwrap();
    assert_eq!(pc.llm.api_key.as_deref(), Some("resolved"));
}
