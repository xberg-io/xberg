use super::*;
use crate::utils::test_support::{child_test_root, run_child_test};

#[test]
fn formula_threads_cap_only_explicit_cpu_at_four_or_more() {
    for budget in [1, 2, 3, 4, 8] {
        let cpu = FormulaSessionKey::new(None, ExecutionProviderType::Cpu, budget);
        assert_eq!(cpu.threads, if budget >= 4 { 2 } else { budget });
        for provider in [
            ExecutionProviderType::Auto,
            ExecutionProviderType::CoreMl,
            ExecutionProviderType::Cuda,
            ExecutionProviderType::TensorRt,
        ] {
            assert_eq!(FormulaSessionKey::new(None, provider, budget).threads, budget);
        }
    }
}

#[test]
fn formula_session_identity_tracks_the_provider_device_and_effective_threads() {
    let cpu = |budget| FormulaSessionKey::new(None, ExecutionProviderType::Cpu, budget);
    assert_eq!(cpu(2), cpu(4));
    assert_ne!(cpu(3), cpu(4));
    assert_ne!(cpu(1), cpu(4));
    assert_ne!(cpu(4), FormulaSessionKey::new(None, ExecutionProviderType::Auto, 2));
    let device = |device_id| {
        let config = AccelerationConfig {
            provider: ExecutionProviderType::Cuda,
            device_id,
        };
        FormulaSessionKey::new(Some(&config), ExecutionProviderType::Cuda, 4)
    };
    assert_ne!(device(0), device(1));
}

const CHILD_CASE: &str = "XBERG_TEST_FORMULA_PROVIDER_CHILD";

#[test]
fn formula_thread_policy_honors_ort_provider_override_precedence() {
    for case in ["cpu", "cuda", "auto", "unset"] {
        run_child_test(
            "formula_recognition::session_policy_tests::provider_override_child",
            CHILD_CASE,
            case,
            |command| {
                if case == "unset" {
                    command.env_remove("XBERG_ORT_EP");
                } else {
                    command.env("XBERG_ORT_EP", case);
                }
            },
        );
    }
}

#[test]
#[ignore = "run with the formula provider test subprocess"]
fn provider_override_child() {
    let Some(case) = child_test_root(CHILD_CASE) else {
        return;
    };
    let (requested, provider, threads) = match case.to_str().expect("formula provider case") {
        "cpu" => (ExecutionProviderType::Cuda, ExecutionProviderType::Cpu, 2),
        "cuda" => (ExecutionProviderType::Cpu, ExecutionProviderType::Cuda, 4),
        "auto" => (ExecutionProviderType::Cpu, ExecutionProviderType::Auto, 4),
        "unset" => (ExecutionProviderType::Cpu, ExecutionProviderType::Cpu, 2),
        _ => panic!("unknown formula provider case"),
    };
    let config = AccelerationConfig {
        provider: requested,
        device_id: 3,
    };
    let key = FormulaSessionKey::resolve(Some(&config), 4);
    assert_eq!(key.provider, provider);
    assert_eq!(key.threads, threads);
    assert_eq!(key.acceleration.as_ref(), Some(&config));
}
