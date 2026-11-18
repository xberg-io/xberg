//! Tests for environment variable overrides.

use crate::core::ServerConfig;
use crate::utils::test_support::{child_test_root, run_child_test};

const CHILD_CASE: &str = "XBERG_SERVER_ENV_CHILD_CASE";

fn run_env_case(case: &str, variables: &[(&str, &str)]) {
    let module = module_path!().strip_prefix("xberg::").expect("crate module prefix");
    let test = format!("{module}::{case}::child");
    run_child_test(&test, CHILD_CASE, case, |command| {
        // Preserve the child process's loader environment while removing ambient Xberg overrides. ~keep
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
    (fn $case:ident [$($var:literal => $value:literal),+ $(,)?] $body:block) => {
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
    fn test_apply_env_host_override [
        "XBERG_HOST" => "192.168.1.1",
    ] {
        let mut config = ServerConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.host, "192.168.1.1");
    }
}

isolated_env_test! {
    fn test_apply_env_port_override [
        "XBERG_PORT" => "5000",
    ] {
        let mut config = ServerConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.port, 5000);
    }
}

isolated_env_test! {
    fn test_apply_env_port_invalid [
        "XBERG_PORT" => "not_a_number",
    ] {
        let mut config = ServerConfig::default();
        let result = config.apply_env_overrides();

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("XBERG_PORT must be a valid u16")
        );
    }
}

isolated_env_test! {
    fn test_apply_env_cors_origins_override [
        "XBERG_CORS_ORIGINS" => "https://example.com, https://other.com",
    ] {
        let mut config = ServerConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.cors_origins.len(), 2);
        assert!(config.cors_origins.contains(&"https://example.com".to_string()));
        assert!(config.cors_origins.contains(&"https://other.com".to_string()));
    }
}

isolated_env_test! {
    fn test_apply_env_max_request_body_bytes_override [
        "XBERG_MAX_REQUEST_BODY_BYTES" => "52428800",
    ] {
        let mut config = ServerConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.max_request_body_bytes, 52_428_800);
    }
}

isolated_env_test! {
    fn test_apply_env_max_multipart_field_bytes_override [
        "XBERG_MAX_MULTIPART_FIELD_BYTES" => "78643200",
    ] {
        let mut config = ServerConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.max_multipart_field_bytes, 78_643_200);
    }
}

isolated_env_test! {
    fn test_apply_env_job_timeout_secs_override [
        "XBERG_JOB_TIMEOUT_SECS" => "120",
    ] {
        let mut config = ServerConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.job_timeout_secs, 120);
    }
}

isolated_env_test! {
    fn test_apply_env_job_timeout_secs_invalid [
        "XBERG_JOB_TIMEOUT_SECS" => "not_a_number",
    ] {
        let mut config = ServerConfig::default();
        let result = config.apply_env_overrides();

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("XBERG_JOB_TIMEOUT_SECS must be a valid u64")
        );
    }
}

isolated_env_test! {
    fn test_apply_env_multiple_overrides [
        "XBERG_HOST" => "0.0.0.0",
        "XBERG_PORT" => "4000",
        "XBERG_CORS_ORIGINS" => "https://api.example.com",
    ] {
        let mut config = ServerConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 4000);
        assert_eq!(config.cors_origins.len(), 1);
        assert_eq!(config.cors_origins[0], "https://api.example.com");
    }
}
