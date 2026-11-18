//! Comprehensive integration tests for ServerConfig precedence order system.
//!
//! Tests verify the precedence order: CLI > Env > File > Default
//! These tests use real config files and environment variables.

#![cfg(feature = "api")]

use std::fs;
use tempfile::tempdir;
use xberg::ServerConfig;
use xberg_child_test::{child_test_root, run_child_test};

const CHILD_CASE: &str = "XBERG_SERVER_CONFIG_INTEGRATION_CHILD";

fn run_env_case(test: &str, variables: &[(&str, &str)]) {
    run_child_test(test, CHILD_CASE, test, |command| {
        // Keep loader and toolchain settings while dropping inherited Xberg overrides. ~keep
        command
            .env_clear()
            .envs(
                std::env::vars_os()
                    .filter(|(key, _)| !key.to_string_lossy().to_ascii_uppercase().starts_with("XBERG_")),
            )
            .env(CHILD_CASE, test)
            .envs(variables.iter().copied());
    });
}

fn check_child_case(test: &str) -> bool {
    let Some(case) = child_test_root(CHILD_CASE) else {
        return false;
    };
    assert_eq!(case.as_os_str(), std::ffi::OsStr::new(test));
    true
}

macro_rules! isolated_env_test {
    (fn $case:ident [$($key:literal => $value:literal),+ $(,)?] $body:block) => {
        #[test]
        fn $case() {
            run_env_case(concat!(stringify!($case), "::child"), &[$(($key, $value)),+]);
        }

        mod $case {
            use super::*;

            #[test]
            #[ignore = "run in an isolated subprocess"]
            fn child() {
                if !check_child_case(concat!(stringify!($case), "::child")) {
                    return;
                }
                $body
            }
        }
    };
}

isolated_env_test! {
    fn test_config_precedence_env_over_file [
        "XBERG_HOST" => "env-host",
        "XBERG_PORT" => "8002",
    ] {
        let dir = tempdir().expect("Operation failed");
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
host = "file-host"
port = 8001
"#,
        )
        .expect("Operation failed");

        let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
        assert_eq!(config.host, "file-host");
        assert_eq!(config.port, 8001);

        config.apply_env_overrides().expect("Operation failed");

        assert_eq!(config.host, "env-host", "Env HOST should override file HOST");
        assert_eq!(config.port, 8002, "Env PORT should override file PORT");
    }
}

#[test]
fn test_file_only_configuration() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
host = "192.168.1.100"
port = 9000
cors_origins = ["https://app.example.com"]
max_request_body_bytes = 50000000
max_multipart_field_bytes = 75000000
"#,
    )
    .expect("Operation failed");

    let config = ServerConfig::from_file(&config_path).expect("Operation failed");

    assert_eq!(config.host, "192.168.1.100");
    assert_eq!(config.port, 9000);
    assert_eq!(config.cors_origins.len(), 1);
    assert_eq!(config.cors_origins[0], "https://app.example.com");
    assert_eq!(config.max_request_body_bytes, 50_000_000);
    assert_eq!(config.max_multipart_field_bytes, 75_000_000);
}

isolated_env_test! {
    fn test_env_only_configuration [
        "XBERG_HOST" => "0.0.0.0",
        "XBERG_PORT" => "3000",
        "XBERG_CORS_ORIGINS" => "https://api.example.com, https://app.example.com",
    ] {
        let mut config = ServerConfig::default();

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8000);

        config.apply_env_overrides().expect("Operation failed");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.cors_origins.len(), 2);
        assert!(config.cors_origins.contains(&"https://api.example.com".to_string()));
        assert!(config.cors_origins.contains(&"https://app.example.com".to_string()));
    }
}

#[test]
fn test_default_configuration() {
    let config = ServerConfig::default();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8000);
    assert!(config.cors_origins.is_empty());
    assert_eq!(config.max_request_body_bytes, 104_857_600);
    assert_eq!(config.max_multipart_field_bytes, 104_857_600);
    assert_eq!(config.listen_addr(), "127.0.0.1:8000");
}

#[test]
fn test_backward_compatibility_no_server_section() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
# No [server] section - extraction-only config
use_cache = false
enable_quality_processing = true
"#,
    )
    .expect("Operation failed");

    let config = ServerConfig::from_file(&config_path).expect("Operation failed");

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8000);
    assert!(config.cors_origins.is_empty());
}

#[test]
fn test_config_format_toml() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
host = "10.0.0.1"
port = 7000
cors_origins = ["https://test.com"]
"#,
    )
    .expect("Operation failed");

    let config = ServerConfig::from_file(&config_path).expect("Operation failed");
    assert_eq!(config.host, "10.0.0.1");
    assert_eq!(config.port, 7000);
}

#[test]
fn test_config_format_yaml() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.yaml");

    fs::write(
        &config_path,
        r#"
host: 10.0.0.2
port: 7001
cors_origins:
  - https://test.com
"#,
    )
    .expect("Operation failed");

    let config = ServerConfig::from_file(&config_path).expect("Operation failed");
    assert_eq!(config.host, "10.0.0.2");
    assert_eq!(config.port, 7001);
}

#[test]
fn test_config_format_json() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.json");

    fs::write(
        &config_path,
        r#"{
  "host": "10.0.0.3",
  "port": 7002,
  "cors_origins": ["https://test.com"]
}
"#,
    )
    .expect("Operation failed");

    let config = ServerConfig::from_file(&config_path).expect("Operation failed");
    assert_eq!(config.host, "10.0.0.3");
    assert_eq!(config.port, 7002);
}

#[test]
fn test_cors_configuration_allow_all() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
host = "127.0.0.1"
port = 8000
# Empty cors_origins means allow all
"#,
    )
    .expect("Operation failed");

    let config = ServerConfig::from_file(&config_path).expect("Operation failed");

    assert!(config.cors_allows_all(), "Empty cors_origins should allow all");
    assert!(config.is_origin_allowed("https://any.com"));
    assert!(config.is_origin_allowed("http://localhost:3000"));
}

#[test]
fn test_cors_configuration_specific_origins() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
host = "127.0.0.1"
port = 8000
cors_origins = ["https://app1.com", "https://app2.com"]
"#,
    )
    .expect("Operation failed");

    let config = ServerConfig::from_file(&config_path).expect("Operation failed");

    assert!(!config.cors_allows_all(), "Specific origins should not allow all");
    assert!(config.is_origin_allowed("https://app1.com"));
    assert!(config.is_origin_allowed("https://app2.com"));
    assert!(!config.is_origin_allowed("https://app3.com"));
}

isolated_env_test! {
    fn test_cors_precedence_env_over_file [
        "XBERG_CORS_ORIGINS" => "https://env1.com, https://env2.com",
    ] {
        let dir = tempdir().expect("Operation failed");
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
cors_origins = ["https://file.com"]
"#,
        )
        .expect("Operation failed");

        let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
        assert_eq!(config.cors_origins.len(), 1);
        assert_eq!(config.cors_origins[0], "https://file.com");

        config.apply_env_overrides().expect("Operation failed");

        assert_eq!(config.cors_origins.len(), 2);
        assert!(config.cors_origins.contains(&"https://env1.com".to_string()));
        assert!(config.cors_origins.contains(&"https://env2.com".to_string()));
    }
}

isolated_env_test! {
    fn test_invalid_env_port [
        "XBERG_PORT" => "not_a_number",
    ] {
        let mut config = ServerConfig::default();
        let result = config.apply_env_overrides();

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("XBERG_PORT"));
        assert!(err_msg.contains("valid u16"));
    }
}

isolated_env_test! {
    fn test_invalid_env_max_request_body_bytes [
        "XBERG_MAX_REQUEST_BODY_BYTES" => "invalid_number",
    ] {
        let mut config = ServerConfig::default();
        let result = config.apply_env_overrides();

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("XBERG_MAX_REQUEST_BODY_BYTES"));
    }
}

isolated_env_test! {
    fn test_partial_overrides_host_only [
        "XBERG_HOST" => "env-host",
    ] {
        let dir = tempdir().expect("Operation failed");
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
host = "file-host"
port = 8001
"#,
        )
        .expect("Operation failed");

        let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
        config.apply_env_overrides().expect("Operation failed");

        assert_eq!(config.host, "env-host", "Host should be overridden by env");
        assert_eq!(config.port, 8001, "Port should keep file value");
    }
}

isolated_env_test! {
    fn test_partial_overrides_port_only [
        "XBERG_PORT" => "9000",
    ] {
        let dir = tempdir().expect("Operation failed");
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
host = "file-host"
port = 8001
"#,
        )
        .expect("Operation failed");

        let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
        config.apply_env_overrides().expect("Operation failed");

        assert_eq!(config.host, "file-host", "Host should keep file value");
        assert_eq!(config.port, 9000, "Port should be overridden by env");
    }
}

isolated_env_test! {
    fn test_complex_scenario_multiple_settings [
        "XBERG_HOST" => "0.0.0.0",
        "XBERG_PORT" => "3000",
        "XBERG_CORS_ORIGINS" => "https://env.com",
    ] {
        let dir = tempdir().expect("Operation failed");
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
host = "127.0.0.1"
port = 8000
cors_origins = ["https://file.com"]
max_request_body_bytes = 50000000
max_multipart_field_bytes = 75000000
"#,
        )
        .expect("Operation failed");

        let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
        config.apply_env_overrides().expect("Operation failed");

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.cors_origins.len(), 1);
        assert_eq!(config.cors_origins[0], "https://env.com");
        assert_eq!(config.max_request_body_bytes, 50_000_000, "File value should persist");
        assert_eq!(config.max_multipart_field_bytes, 75_000_000);
    }
}

#[test]
fn test_listen_addr_helper() {
    let mut config = ServerConfig::default();
    assert_eq!(config.listen_addr(), "127.0.0.1:8000");

    config.host = "0.0.0.0".to_string();
    config.port = 3000;
    assert_eq!(config.listen_addr(), "0.0.0.0:3000");
}

#[test]
fn test_upload_limits_to_mb_conversion() {
    let mut config = ServerConfig::default();

    assert_eq!(config.max_request_body_mb(), 100);

    config.max_request_body_bytes = 1_048_576;
    assert_eq!(config.max_request_body_mb(), 1);

    config.max_request_body_bytes = 1_048_577;
    assert_eq!(config.max_request_body_mb(), 2);

    config.max_multipart_field_bytes = 1_048_576;
    assert_eq!(config.max_multipart_field_mb(), 1);

    config.max_multipart_field_bytes = 52_428_800;
    assert_eq!(config.max_multipart_field_mb(), 50);
}

#[test]
fn test_serialization_consistency() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    let original = r#"
host = "192.168.1.100"
port = 9000
cors_origins = ["https://app.com"]
max_request_body_bytes = 50000000
max_multipart_field_bytes = 75000000
"#;

    fs::write(&config_path, original).expect("Operation failed");

    let config = ServerConfig::from_file(&config_path).expect("Operation failed");

    let serialized = toml::to_string(&config).expect("Operation failed");

    let config2: ServerConfig = toml::from_str(&serialized).expect("Failed to parse string");

    assert_eq!(config.host, config2.host);
    assert_eq!(config.port, config2.port);
    assert_eq!(config.cors_origins, config2.cors_origins);
    assert_eq!(config.max_request_body_bytes, config2.max_request_body_bytes);
    assert_eq!(config.max_multipart_field_bytes, config2.max_multipart_field_bytes);
}

isolated_env_test! {
    fn test_empty_cors_to_specific_via_env [
        "XBERG_CORS_ORIGINS" => "https://restricted.com",
    ] {
        let dir = tempdir().expect("Operation failed");
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
host = "127.0.0.1"
port = 8000
"#,
        )
        .expect("Operation failed");

        let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
        assert!(config.cors_allows_all(), "File config allows all origins");

        config.apply_env_overrides().expect("Operation failed");

        assert!(!config.cors_allows_all(), "Should now restrict to specific origin");
        assert!(config.is_origin_allowed("https://restricted.com"));
        assert!(!config.is_origin_allowed("https://other.com"));
    }
}

#[test]
fn test_max_limits_across_formats() {
    let dir = tempdir().expect("Operation failed");

    let toml_path = dir.path().join("config.toml");
    fs::write(
        &toml_path,
        r#"
max_request_body_bytes = 100000000
max_multipart_field_bytes = 200000000
"#,
    )
    .expect("Operation failed");

    let toml_config = ServerConfig::from_file(&toml_path).expect("Operation failed");
    assert_eq!(toml_config.max_request_body_bytes, 100_000_000);
    assert_eq!(toml_config.max_multipart_field_bytes, 200_000_000);

    let yaml_path = dir.path().join("config.yaml");
    fs::write(
        &yaml_path,
        r#"
max_request_body_bytes: 100000000
max_multipart_field_bytes: 200000000
"#,
    )
    .expect("Operation failed");

    let yaml_config = ServerConfig::from_file(&yaml_path).expect("Operation failed");
    assert_eq!(yaml_config.max_request_body_bytes, 100_000_000);
    assert_eq!(yaml_config.max_multipart_field_bytes, 200_000_000);

    let json_path = dir.path().join("config.json");
    fs::write(
        &json_path,
        r#"{
  "max_request_body_bytes": 100000000,
  "max_multipart_field_bytes": 200000000
}
"#,
    )
    .expect("Operation failed");

    let json_config = ServerConfig::from_file(&json_path).expect("Operation failed");
    assert_eq!(json_config.max_request_body_bytes, 100_000_000);
    assert_eq!(json_config.max_multipart_field_bytes, 200_000_000);
}

/// Port 0 asks the OS for an arbitrary ephemeral port, which defeats the point of a
/// configured listen address (see `MIN_PORT` in `core::config_validation::dependencies`).
/// `validate_port` deliberately rejects it, so `XBERG_PORT=0` must fail the same way a
/// config-file `port = 0` does (`should_reject_config_when_port_is_zero` in
/// `core::server_config::validation`). This test intentionally asserts rejection, not
/// acceptance.
#[test]
fn test_port_validation_bounds() {
    run_env_case("port_validation::zero", &[("XBERG_PORT", "0")]);
    run_env_case("port_validation::max", &[("XBERG_PORT", "65535")]);
    run_env_case("port_validation::overflow", &[("XBERG_PORT", "65536")]);
}

mod port_validation {
    use super::*;

    #[test]
    #[ignore = "run in an isolated subprocess"]
    fn zero() {
        if !check_child_case("port_validation::zero") {
            return;
        }
        let mut config = ServerConfig::default();
        let result = config.apply_env_overrides();
        assert_eq!(
            result.unwrap_err().to_string(),
            "Validation error: Port must be 1-65535, got 0. \
         Set 'server.port' (or XBERG_PORT) to a free port such as 8000."
        );
    }

    #[test]
    #[ignore = "run in an isolated subprocess"]
    fn max() {
        if !check_child_case("port_validation::max") {
            return;
        }
        let mut config = ServerConfig::default();
        config.apply_env_overrides().expect("Operation failed");
        assert_eq!(config.port, 65535);
    }

    #[test]
    #[ignore = "run in an isolated subprocess"]
    fn overflow() {
        if !check_child_case("port_validation::overflow") {
            return;
        }
        let mut config = ServerConfig::default();
        let result = config.apply_env_overrides();
        assert!(result.is_err());
    }
}

isolated_env_test! {
    fn test_multiple_env_overrides_simultaneous [
        "XBERG_HOST" => "192.168.1.1",
        "XBERG_PORT" => "5000",
        "XBERG_CORS_ORIGINS" => "https://api.com, https://app.com",
        "XBERG_MAX_REQUEST_BODY_BYTES" => "150000000",
        "XBERG_MAX_MULTIPART_FIELD_BYTES" => "250000000",
    ] {
        let dir = tempdir().expect("Operation failed");
        let config_path = dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
host = "127.0.0.1"
port = 8000
"#,
        )
        .expect("Operation failed");

        let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
        config.apply_env_overrides().expect("Operation failed");

        assert_eq!(config.host, "192.168.1.1");
        assert_eq!(config.port, 5000);
        assert_eq!(config.cors_origins.len(), 2);
        assert_eq!(config.max_request_body_bytes, 150_000_000);
        assert_eq!(config.max_multipart_field_bytes, 250_000_000);
    }
}
