//! Comprehensive integration tests for ServerConfig precedence order system.
//!
//! Tests verify the precedence order: CLI > Env > File > Default
//! These tests use real config files and environment variables.

#![cfg(feature = "api")]

use kreuzberg::ServerConfig;
use std::fs;
use tempfile::tempdir;

// Helper function to cleanup environment variables
#[allow(unsafe_code)]
fn cleanup_env_vars() {
    unsafe {
        std::env::remove_var("KREUZBERG_HOST");
        std::env::remove_var("KREUZBERG_PORT");
        std::env::remove_var("KREUZBERG_CORS_ORIGINS");
        std::env::remove_var("KREUZBERG_MAX_REQUEST_BODY_BYTES");
        std::env::remove_var("KREUZBERG_MAX_MULTIPART_FIELD_BYTES");
    }
}

// Helper function to set environment variables
#[allow(unsafe_code)]
fn set_env(key: &str, value: &str) {
    unsafe {
        std::env::set_var(key, value);
    }
}

// Helper function to get and store original environment variables
fn save_env(keys: &[&str]) -> Vec<(String, Option<String>)> {
    keys.iter()
        .map(|key| (key.to_string(), std::env::var(key).ok()))
        .collect()
}

// Helper function to restore environment variables
#[allow(unsafe_code)]
fn restore_env(saved: Vec<(String, Option<String>)>) {
    unsafe {
        for (key, value) in saved {
            if let Some(v) = value {
                std::env::set_var(&key, v);
            } else {
                std::env::remove_var(&key);
            }
        }
    }
}

// Test 1: Config precedence order - Env wins over File
#[test]
#[serial_test::serial]
fn test_config_precedence_env_over_file() {
    let saved = save_env(&["KREUZBERG_HOST", "KREUZBERG_PORT"]);

    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    // Create config file with file values
    fs::write(
        &config_path,
        r#"
host = "file-host"
port = 8001
"#,
    )
    .expect("Operation failed");

    // Set env vars (should override file)
    set_env("KREUZBERG_HOST", "env-host");
    set_env("KREUZBERG_PORT", "8002");

    // Load and apply
    let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
    assert_eq!(config.host, "file-host");
    assert_eq!(config.port, 8001);

    // Apply env overrides
    config.apply_env_overrides().expect("Operation failed");

    // Verify env vars won (Env > File)
    assert_eq!(config.host, "env-host", "Env HOST should override file HOST");
    assert_eq!(config.port, 8002, "Env PORT should override file PORT");

    cleanup_env_vars();
    restore_env(saved);
}

// Test 2: File-only configuration
#[test]
fn test_file_only_configuration() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    // Create config with specific values
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

// Test 3: Env-only configuration (no config file)
#[test]
#[serial_test::serial]
fn test_env_only_configuration() {
    let saved = save_env(&["KREUZBERG_HOST", "KREUZBERG_PORT", "KREUZBERG_CORS_ORIGINS"]);

    set_env("KREUZBERG_HOST", "0.0.0.0");
    set_env("KREUZBERG_PORT", "3000");
    set_env(
        "KREUZBERG_CORS_ORIGINS",
        "https://api.example.com, https://app.example.com",
    );

    // Create default config
    let mut config = ServerConfig::default();

    // Verify defaults initially
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8000);

    // Apply env overrides
    config.apply_env_overrides().expect("Operation failed");

    // Verify env vars are used
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.cors_origins.len(), 2);
    assert!(config.cors_origins.contains(&"https://api.example.com".to_string()));
    assert!(config.cors_origins.contains(&"https://app.example.com".to_string()));

    cleanup_env_vars();
    restore_env(saved);
}

// Test 4: Default configuration
#[test]
fn test_default_configuration() {
    let config = ServerConfig::default();

    // Verify defaults
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8000);
    assert!(config.cors_origins.is_empty());
    assert_eq!(config.max_request_body_bytes, 104_857_600); // 100 MB
    assert_eq!(config.max_multipart_field_bytes, 104_857_600); // 100 MB
    assert_eq!(config.listen_addr(), "127.0.0.1:8000");
}

// Test 5: Backward compatibility - file without [server] section
#[test]
fn test_backward_compatibility_no_server_section() {
    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    // Create config with only extraction settings (no [server] section)
    fs::write(
        &config_path,
        r#"
# No [server] section - extraction-only config
use_cache = false
enable_quality_processing = true
"#,
    )
    .expect("Operation failed");

    // ServerConfig::from_file should load with defaults for missing [server] section
    let config = ServerConfig::from_file(&config_path).expect("Operation failed");

    // Verify ServerConfig fields have defaults
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8000);
    assert!(config.cors_origins.is_empty());
}

// Test 6: All three formats - TOML
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

// Test 7: All three formats - YAML
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

// Test 8: All three formats - JSON
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

// Test 9: CORS configuration - empty (allow all)
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

// Test 10: CORS configuration - specific origins
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

// Test 11: CORS precedence - env overrides file
#[test]
#[serial_test::serial]
fn test_cors_precedence_env_over_file() {
    let saved = save_env(&["KREUZBERG_CORS_ORIGINS"]);

    let dir = tempdir().expect("Operation failed");
    let config_path = dir.path().join("config.toml");

    fs::write(
        &config_path,
        r#"
cors_origins = ["https://file.com"]
"#,
    )
    .expect("Operation failed");

    set_env("KREUZBERG_CORS_ORIGINS", "https://env1.com, https://env2.com");

    let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
    assert_eq!(config.cors_origins.len(), 1);
    assert_eq!(config.cors_origins[0], "https://file.com");

    config.apply_env_overrides().expect("Operation failed");

    assert_eq!(config.cors_origins.len(), 2);
    assert!(config.cors_origins.contains(&"https://env1.com".to_string()));
    assert!(config.cors_origins.contains(&"https://env2.com".to_string()));

    cleanup_env_vars();
    restore_env(saved);
}

// Test 14: Invalid env var values - invalid port
#[test]
#[serial_test::serial]
fn test_invalid_env_port() {
    let saved = save_env(&["KREUZBERG_PORT"]);

    set_env("KREUZBERG_PORT", "not_a_number");

    let mut config = ServerConfig::default();
    let result = config.apply_env_overrides();

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("KREUZBERG_PORT"));
    assert!(err_msg.contains("valid u16"));

    cleanup_env_vars();
    restore_env(saved);
}

// Test 15: Invalid env var values - invalid max_request_body_bytes
#[test]
#[serial_test::serial]
fn test_invalid_env_max_request_body_bytes() {
    let saved = save_env(&["KREUZBERG_MAX_REQUEST_BODY_BYTES"]);

    set_env("KREUZBERG_MAX_REQUEST_BODY_BYTES", "invalid_number");

    let mut config = ServerConfig::default();
    let result = config.apply_env_overrides();

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("KREUZBERG_MAX_REQUEST_BODY_BYTES"));

    cleanup_env_vars();
    restore_env(saved);
}

// Test 16: Partial overrides - only host, not port
#[test]
#[serial_test::serial]
fn test_partial_overrides_host_only() {
    let saved = save_env(&["KREUZBERG_HOST", "KREUZBERG_PORT"]);

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

    set_env("KREUZBERG_HOST", "env-host");
    // Explicitly don't set KREUZBERG_PORT

    let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
    config.apply_env_overrides().expect("Operation failed");

    assert_eq!(config.host, "env-host", "Host should be overridden by env");
    assert_eq!(config.port, 8001, "Port should keep file value");

    cleanup_env_vars();
    restore_env(saved);
}

// Test 17: Partial overrides - only port, not host
#[test]
#[serial_test::serial]
fn test_partial_overrides_port_only() {
    let saved = save_env(&["KREUZBERG_HOST", "KREUZBERG_PORT"]);

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

    set_env("KREUZBERG_PORT", "9000");
    // Explicitly don't set KREUZBERG_HOST

    let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
    config.apply_env_overrides().expect("Operation failed");

    assert_eq!(config.host, "file-host", "Host should keep file value");
    assert_eq!(config.port, 9000, "Port should be overridden by env");

    cleanup_env_vars();
    restore_env(saved);
}

// Test 18: Complex scenario with multiple settings
#[test]
#[serial_test::serial]
fn test_complex_scenario_multiple_settings() {
    let saved = save_env(&[
        "KREUZBERG_HOST",
        "KREUZBERG_PORT",
        "KREUZBERG_CORS_ORIGINS",
        "KREUZBERG_MAX_REQUEST_BODY_BYTES",
    ]);

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

    // Override some settings
    set_env("KREUZBERG_HOST", "0.0.0.0");
    set_env("KREUZBERG_PORT", "3000");
    set_env("KREUZBERG_CORS_ORIGINS", "https://env.com");
    // Don't set max_request_body_bytes - should keep file value

    let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
    config.apply_env_overrides().expect("Operation failed");

    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.cors_origins.len(), 1);
    assert_eq!(config.cors_origins[0], "https://env.com");
    assert_eq!(config.max_request_body_bytes, 50_000_000, "File value should persist");
    assert_eq!(config.max_multipart_field_bytes, 75_000_000);

    cleanup_env_vars();
    restore_env(saved);
}

// Test 19: listen_addr helper method
#[test]
fn test_listen_addr_helper() {
    let mut config = ServerConfig::default();
    assert_eq!(config.listen_addr(), "127.0.0.1:8000");

    config.host = "0.0.0.0".to_string();
    config.port = 3000;
    assert_eq!(config.listen_addr(), "0.0.0.0:3000");
}

// Test 20: Upload limits conversion to MB
#[test]
fn test_upload_limits_to_mb_conversion() {
    let mut config = ServerConfig::default();

    // Test request body MB
    assert_eq!(config.max_request_body_mb(), 100);

    config.max_request_body_bytes = 1_048_576; // 1 MB
    assert_eq!(config.max_request_body_mb(), 1);

    config.max_request_body_bytes = 1_048_577; // 1 MB + 1 byte - should round up
    assert_eq!(config.max_request_body_mb(), 2);

    // Test multipart field MB
    config.max_multipart_field_bytes = 1_048_576;
    assert_eq!(config.max_multipart_field_mb(), 1);

    config.max_multipart_field_bytes = 52_428_800; // 50 MB
    assert_eq!(config.max_multipart_field_mb(), 50);
}

// Test 21: Serialization consistency
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

    // Serialize back
    let serialized = toml::to_string(&config).expect("Operation failed");

    // Deserialize again
    let config2: ServerConfig = toml::from_str(&serialized).expect("Failed to parse string");

    // Verify consistency
    assert_eq!(config.host, config2.host);
    assert_eq!(config.port, config2.port);
    assert_eq!(config.cors_origins, config2.cors_origins);
    assert_eq!(config.max_request_body_bytes, config2.max_request_body_bytes);
    assert_eq!(config.max_multipart_field_bytes, config2.max_multipart_field_bytes);
}

// Test 22: Empty CORS origins with env override
#[test]
#[serial_test::serial]
fn test_empty_cors_to_specific_via_env() {
    let saved = save_env(&["KREUZBERG_CORS_ORIGINS"]);

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

    // Override with specific origins
    set_env("KREUZBERG_CORS_ORIGINS", "https://restricted.com");
    config.apply_env_overrides().expect("Operation failed");

    assert!(!config.cors_allows_all(), "Should now restrict to specific origin");
    assert!(config.is_origin_allowed("https://restricted.com"));
    assert!(!config.is_origin_allowed("https://other.com"));

    cleanup_env_vars();
    restore_env(saved);
}

// Test 23: Max upload limits in different formats
#[test]
fn test_max_limits_across_formats() {
    let dir = tempdir().expect("Operation failed");

    // Test TOML
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

    // Test YAML
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

    // Test JSON
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

// Test 24: Port validation at bounds
#[test]
#[serial_test::serial]
fn test_port_validation_bounds() {
    let saved = save_env(&["KREUZBERG_PORT"]);

    // Valid port: 0
    set_env("KREUZBERG_PORT", "0");
    let mut config = ServerConfig::default();
    config.apply_env_overrides().expect("Operation failed");
    assert_eq!(config.port, 0);

    // Valid port: 65535 (max u16)
    set_env("KREUZBERG_PORT", "65535");
    let mut config = ServerConfig::default();
    config.apply_env_overrides().expect("Operation failed");
    assert_eq!(config.port, 65535);

    // Invalid port: too large
    set_env("KREUZBERG_PORT", "65536");
    let mut config = ServerConfig::default();
    let result = config.apply_env_overrides();
    assert!(result.is_err());

    cleanup_env_vars();
    restore_env(saved);
}

// Test 25: Multiple env var overrides at once
#[test]
#[serial_test::serial]
fn test_multiple_env_overrides_simultaneous() {
    let saved = save_env(&[
        "KREUZBERG_HOST",
        "KREUZBERG_PORT",
        "KREUZBERG_CORS_ORIGINS",
        "KREUZBERG_MAX_REQUEST_BODY_BYTES",
        "KREUZBERG_MAX_MULTIPART_FIELD_BYTES",
    ]);

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

    // Set all env vars
    set_env("KREUZBERG_HOST", "192.168.1.1");
    set_env("KREUZBERG_PORT", "5000");
    set_env("KREUZBERG_CORS_ORIGINS", "https://api.com, https://app.com");
    set_env("KREUZBERG_MAX_REQUEST_BODY_BYTES", "150000000");
    set_env("KREUZBERG_MAX_MULTIPART_FIELD_BYTES", "250000000");

    let mut config = ServerConfig::from_file(&config_path).expect("Operation failed");
    config.apply_env_overrides().expect("Operation failed");

    // All should be overridden
    assert_eq!(config.host, "192.168.1.1");
    assert_eq!(config.port, 5000);
    assert_eq!(config.cors_origins.len(), 2);
    assert_eq!(config.max_request_body_bytes, 150_000_000);
    assert_eq!(config.max_multipart_field_bytes, 250_000_000);

    cleanup_env_vars();
    restore_env(saved);
}
