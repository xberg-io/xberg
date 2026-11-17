use super::super::*;
use xberg::ExtractionConfig;

use crate::commands::env_test;

const CHILD: &str = "commands::overrides::tests::llm_api_key::llm_api_key_env_child";
const ENV_TEST_VALUE: &str = "env-value";
const LLM_API_KEY_VAR: &str = "XBERG_LLM_API_KEY";

fn run_key_case(case: &str, value: Option<&str>) {
    env_test::run_child_test(CHILD, env_test::CASE, case, |command| {
        command.env_remove(LLM_API_KEY_VAR);
        if let Some(value) = value {
            command.env(LLM_API_KEY_VAR, value);
        }
    });
}

#[test]
fn cli_flag_takes_precedence_over_env_var() {
    run_key_case("cli-wins", Some(ENV_TEST_VALUE));
}

#[test]
fn env_var_used_when_cli_flag_absent() {
    run_key_case("env-fallback", Some(ENV_TEST_VALUE));
}

#[test]
fn returns_none_when_neither_source_is_set() {
    run_key_case("absent", None);
}

#[test]
fn empty_cli_flag_does_not_count() {
    run_key_case("empty-cli", Some(ENV_TEST_VALUE));
}

#[test]
#[should_panic(expected = "LLM API key expectation failed")]
fn llm_api_key_child_reports_a_wrong_expectation() {
    run_key_case("wrong-expectation", None);
}

#[test]
#[ignore = "run with an isolated environment by its parent test"]
fn llm_api_key_env_child() {
    let Some(case) = env_test::child_test_root(env_test::CASE) else {
        return;
    };
    let (cli_value, expected_input, expected_key) = match case.to_str().expect("non-UTF-8 case selector") {
        "cli-wins" => (Some("cli-value"), Some(ENV_TEST_VALUE), Some("cli-value")),
        "env-fallback" => (None, Some(ENV_TEST_VALUE), Some(ENV_TEST_VALUE)),
        "absent" => (None, None, None),
        "empty-cli" => (Some("   "), Some(ENV_TEST_VALUE), Some(ENV_TEST_VALUE)),
        "wrong-expectation" => (None, None, Some(ENV_TEST_VALUE)),
        _ => panic!("unknown LLM API key test case"),
    };
    let input = std::env::var_os(LLM_API_KEY_VAR);
    assert!(
        input.as_deref() == expected_input.map(std::ffi::OsStr::new),
        "expected synthetic LLM API key input was not installed"
    );
    assert!(
        resolve_llm_api_key(cli_value).as_deref() == expected_key,
        "LLM API key expectation failed"
    );
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
