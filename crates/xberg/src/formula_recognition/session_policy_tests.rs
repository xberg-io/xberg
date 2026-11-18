use super::*;
use crate::utils::test_support::{child_test_root, run_child_test};

#[test]
fn warm_formula_recognizer_lifetime_depends_on_requested_acceleration() {
    let cpu = AccelerationConfig {
        provider: ExecutionProviderType::Cpu,
        ..Default::default()
    };
    let cuda = AccelerationConfig {
        provider: ExecutionProviderType::Cuda,
        ..Default::default()
    };
    let next_device = AccelerationConfig {
        device_id: 1,
        ..cuda.clone()
    };

    assert!(recognizer_needs_rebuild(None, None));
    assert!(!recognizer_needs_rebuild(Some(None), None));
    assert!(recognizer_needs_rebuild(Some(None), Some(&cpu)));
    assert!(!recognizer_needs_rebuild(Some(Some(&cpu)), Some(&cpu.clone())));
    assert!(recognizer_needs_rebuild(Some(Some(&cpu)), Some(&cuda)));
    assert!(recognizer_needs_rebuild(Some(Some(&cuda)), Some(&next_device)));
}

const CHILD_CASE: &str = "XBERG_TEST_FORMULA_PROVIDER_CHILD";

#[test]
fn formula_session_construction_caps_only_explicit_cpu_at_four_or_more() {
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
    match case.to_str().expect("formula provider case") {
        "cpu" => assert_session_budget(ExecutionProviderType::Cuda, true),
        "cuda" | "auto" => assert_session_budget(ExecutionProviderType::Cpu, false),
        "unset" => {
            assert_session_budget(ExecutionProviderType::Cpu, true);
            for provider in [
                ExecutionProviderType::Auto,
                ExecutionProviderType::CoreMl,
                ExecutionProviderType::Cuda,
                ExecutionProviderType::TensorRt,
            ] {
                assert_session_budget(provider, false);
            }
            assert_eq!(session_threads(None, 4), 4);
        }
        _ => panic!("unknown formula provider case"),
    }
}

fn assert_session_budget(provider: ExecutionProviderType, capped: bool) {
    let acceleration = AccelerationConfig { provider, device_id: 3 };
    for budget in [1, 2, 3, 4, 8] {
        assert_eq!(
            session_threads(Some(&acceleration), budget),
            if capped && budget >= 4 { 2 } else { budget }
        );
    }
}
