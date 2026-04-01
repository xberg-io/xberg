//! Tests for file loading functionality.

use crate::core::ServerConfig;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_from_toml_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.toml");

    fs::write(
        &config_path,
        r#"
host = "0.0.0.0"
port = 3000
cors_origins = ["https://example.com", "https://other.com"]
max_request_body_bytes = 50000000
max_multipart_field_bytes = 75000000
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_toml_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.cors_origins.len(), 2);
    assert_eq!(config.max_request_body_bytes, 50_000_000);
    assert_eq!(config.max_multipart_field_bytes, 75_000_000);
}

#[test]
fn test_from_yaml_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.yaml");

    fs::write(
        &config_path,
        r#"
host: 0.0.0.0
port: 3000
cors_origins:
  - https://example.com
  - https://other.com
max_request_body_bytes: 50000000
max_multipart_field_bytes: 75000000
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_yaml_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.cors_origins.len(), 2);
    assert_eq!(config.max_request_body_bytes, 50_000_000);
    assert_eq!(config.max_multipart_field_bytes, 75_000_000);
}

#[test]
fn test_from_json_file() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.json");

    fs::write(
        &config_path,
        r#"{
  "host": "0.0.0.0",
  "port": 3000,
  "cors_origins": ["https://example.com", "https://other.com"],
  "max_request_body_bytes": 50000000,
  "max_multipart_field_bytes": 75000000
}
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_json_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.cors_origins.len(), 2);
    assert_eq!(config.max_request_body_bytes, 50_000_000);
    assert_eq!(config.max_multipart_field_bytes, 75_000_000);
}

#[test]
fn test_from_file_auto_detects_toml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.toml");

    fs::write(
        &config_path,
        r#"
host = "0.0.0.0"
port = 3000
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
}

#[test]
fn test_from_file_auto_detects_yaml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.yaml");

    fs::write(
        &config_path,
        r#"
host: 0.0.0.0
port: 3000
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
}

#[test]
fn test_from_file_auto_detects_json() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.json");

    fs::write(&config_path, r#"{"host": "0.0.0.0", "port": 3000}"#).unwrap();

    let config = ServerConfig::from_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
}

#[test]
fn test_from_file_unsupported_extension() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.txt");

    fs::write(&config_path, "host = 0.0.0.0").unwrap();

    let result = ServerConfig::from_file(&config_path);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unsupported config file format")
    );
}

#[test]
fn test_from_file_no_extension() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server");

    fs::write(&config_path, "host = 0.0.0.0").unwrap();

    let result = ServerConfig::from_file(&config_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no extension found"));
}

#[test]
fn test_cors_origins_empty_in_toml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.toml");

    fs::write(
        &config_path,
        r#"
host = "127.0.0.1"
port = 8000
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_toml_file(&config_path).unwrap();
    assert!(config.cors_origins.is_empty());
    assert!(config.cors_allows_all());
}

#[test]
fn test_full_configuration_toml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.toml");

    fs::write(
        &config_path,
        r#"
host = "192.168.1.100"
port = 9000
cors_origins = ["https://app1.com", "https://app2.com", "https://app3.com"]
max_request_body_bytes = 200000000
max_multipart_field_bytes = 150000000
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_toml_file(&config_path).unwrap();
    assert_eq!(config.host, "192.168.1.100");
    assert_eq!(config.port, 9000);
    assert_eq!(config.listen_addr(), "192.168.1.100:9000");
    assert_eq!(config.cors_origins.len(), 3);
    assert!(!config.cors_allows_all());
    assert!(config.is_origin_allowed("https://app1.com"));
    assert!(!config.is_origin_allowed("https://app4.com"));
    assert_eq!(config.max_request_body_bytes, 200_000_000);
    assert_eq!(config.max_multipart_field_bytes, 150_000_000);
    assert_eq!(config.max_request_body_mb(), 191);
    assert_eq!(config.max_multipart_field_mb(), 144);
}

#[test]
fn test_from_file_with_nested_server_section_toml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("kreuzberg.toml");

    // Config file with [server] section and other sections (like ExtractionConfig)
    fs::write(
        &config_path,
        r#"
[server]
host = "0.0.0.0"
port = 3000
cors_origins = ["https://example.com"]

[ocr]
backend = "tesseract"
language = "eng"

[extraction]
enabled = true
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.cors_origins.len(), 1);
    assert_eq!(config.cors_origins[0], "https://example.com");
}

#[test]
fn test_from_file_with_nested_server_section_yaml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("kreuzberg.yaml");

    // Config file with server: section and other sections
    fs::write(
        &config_path,
        r#"
server:
  host: 0.0.0.0
  port: 4000
  cors_origins:
    - https://example.com

ocr:
  backend: tesseract
  language: eng
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 4000);
    assert_eq!(config.cors_origins.len(), 1);
}

#[test]
fn test_from_file_with_nested_server_section_json() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("kreuzberg.json");

    // Config file with "server" key and other sections
    fs::write(
        &config_path,
        r#"
{
  "server": {
    "host": "0.0.0.0",
    "port": 5000,
    "cors_origins": ["https://example.com"]
  },
  "ocr": {
    "backend": "tesseract",
    "language": "eng"
  }
}
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_file(&config_path).unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 5000);
    assert_eq!(config.cors_origins.len(), 1);
}

#[test]
fn test_from_file_flat_format_still_works() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("server.toml");

    // Old flat format without [server] section
    fs::write(
        &config_path,
        r#"
host = "192.168.1.1"
port = 6000
        "#,
    )
    .unwrap();

    let config = ServerConfig::from_file(&config_path).unwrap();
    assert_eq!(config.host, "192.168.1.1");
    assert_eq!(config.port, 6000);
}
