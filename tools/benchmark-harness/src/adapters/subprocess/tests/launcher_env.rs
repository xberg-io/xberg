//! The environment xberg's shipped launchers set (ADR 0022) reaches xberg processes only.

use crate::adapters::xberg::launcher_env;

const MAGAZINES: &str = "MallocMaxMagazines";

fn one_magazine() -> (String, String) {
    (MAGAZINES.to_string(), "1".to_string())
}

#[test]
fn launcher_env_sets_one_magazine_only_on_macos() {
    let env = launcher_env(|_| false);
    if cfg!(target_os = "macos") {
        assert_eq!(env, vec![one_magazine()]);
    } else {
        assert!(env.is_empty(), "{env:?}");
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::super::support::{default_run_timeout, test_build_identity};
    use super::{MAGAZINES, launcher_env, one_magazine};
    use crate::adapter::FrameworkAdapter;
    use crate::adapters::subprocess::SubprocessAdapter;
    use crate::adapters::xberg::{AutoOcrBackend, adapter_with_identity, auto_ocr_adapter_with_identity};
    use crate::types::{OutputFormat, XbergPdfBackend, XbergPipeline};

    #[test]
    fn launcher_env_keeps_a_value_the_user_already_set() {
        assert!(launcher_env(|name| name == MAGAZINES).is_empty());
        assert_eq!(launcher_env(|name| name == "PATH"), vec![one_magazine()]);
    }

    /// Every xberg subprocess adapter the harness builds (single-file, CLI batch, auto-OCR)
    /// carries the launcher environment beside its own variables; a competitor adapter, built as
    /// `external.rs` builds them, carries none. ~keep
    #[test]
    fn xberg_command_env_has_one_magazine_and_a_competitor_env_does_not() {
        let user_set = std::env::var_os(MAGAZINES).is_some();
        let expected_injected = if user_set { Vec::new() } else { vec![one_magazine()] };
        let mut xberg_adapters = Vec::new();
        for batch in [false, true] {
            xberg_adapters.push(
                adapter_with_identity(
                    XbergPipeline::Baseline,
                    OutputFormat::Markdown,
                    batch,
                    false,
                    XbergPdfBackend::Native,
                    default_run_timeout(),
                    test_build_identity(),
                )
                .unwrap(),
            );
            xberg_adapters.push(
                auto_ocr_adapter_with_identity(
                    AutoOcrBackend::Tesseract,
                    OutputFormat::Markdown,
                    batch,
                    default_run_timeout(),
                    test_build_identity(),
                )
                .unwrap(),
            );
        }
        for adapter in &xberg_adapters {
            assert_eq!(adapter.env().contains(&one_magazine()), !user_set, "{}", adapter.name());
            assert!(
                adapter.env().iter().any(|(key, _)| key == "XBERG_EMIT_STAGE_TIMING"),
                "{}",
                adapter.name()
            );
            assert_eq!(adapter.injected_xberg_env(), expected_injected, "{}", adapter.name());
        }

        let competitor = SubprocessAdapter::new("docling", "python3", vec![], vec![], vec!["pdf".to_string()]);
        assert!(!competitor.env().iter().any(|(key, _)| key == MAGAZINES));
        assert!(competitor.injected_xberg_env().is_empty());
    }
}

#[cfg(unix)]
mod spawned {
    use super::MAGAZINES;
    use crate::adapters::subprocess::SubprocessAdapter;
    use crate::types::OutputFormat;
    use std::time::Duration;

    fn probe(name: &str) -> SubprocessAdapter {
        SubprocessAdapter::new(
            name,
            "sh",
            vec!["-c".to_string(), format!("printf '%s' \"${{{MAGAZINES}-unset}}\"")],
            vec![],
            vec!["pdf".to_string()],
        )
    }

    async fn spawned_magazines(adapter: &SubprocessAdapter) -> String {
        let input = tempfile::NamedTempFile::new().unwrap();
        let execution = adapter
            .execute_subprocess(
                input.path(),
                Duration::from_secs(10),
                false,
                None,
                OutputFormat::Markdown,
            )
            .await
            .unwrap();
        assert!(execution.error.is_none(), "{:?}", execution.error);
        execution.stdout
    }

    /// The injected variable reaches the measured child through the start-barrier `sh`, and an
    /// adapter without it leaves the child with the harness's own environment. ~keep
    #[tokio::test]
    async fn launcher_env_reaches_the_spawned_process_only_when_injected() {
        let inherited = std::env::var(MAGAZINES).unwrap_or_else(|_| "unset".to_string());

        let injected = probe("xberg-env-probe").with_launcher_env(vec![(MAGAZINES.to_string(), "7".to_string())]);
        assert_eq!(spawned_magazines(&injected).await, "7");
        assert_eq!(spawned_magazines(&probe("docling")).await, inherited);
    }
}
