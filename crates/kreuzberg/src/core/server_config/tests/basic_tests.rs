//! Basic tests for ServerConfig functionality.

use crate::core::ServerConfig;

#[test]
fn test_default_config() {
    let config = ServerConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8000);
    assert!(config.cors_origins.is_empty());
    assert_eq!(config.max_request_body_bytes, 104_857_600);
    assert_eq!(config.max_multipart_field_bytes, 104_857_600);
}

#[test]
fn test_listen_addr() {
    let config = ServerConfig::default();
    assert_eq!(config.listen_addr(), "127.0.0.1:8000");
}

#[test]
fn test_listen_addr_custom() {
    let config = ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
        ..Default::default()
    };
    assert_eq!(config.listen_addr(), "0.0.0.0:3000");
}

#[test]
fn test_cors_allows_all() {
    let mut config = ServerConfig::default();
    assert!(config.cors_allows_all());

    config.cors_origins.push("https://example.com".to_string());
    assert!(!config.cors_allows_all());
}

#[test]
fn test_is_origin_allowed_all() {
    let config = ServerConfig::default();
    assert!(config.is_origin_allowed("https://example.com"));
    assert!(config.is_origin_allowed("https://other.com"));
}

#[test]
fn test_is_origin_allowed_specific() {
    let mut config = ServerConfig::default();
    config.cors_origins.push("https://allowed.com".to_string());

    assert!(config.is_origin_allowed("https://allowed.com"));
    assert!(!config.is_origin_allowed("https://denied.com"));
}

#[test]
fn test_max_request_body_mb() {
    let config = ServerConfig::default();
    assert_eq!(config.max_request_body_mb(), 100);
}

#[test]
fn test_max_multipart_field_mb() {
    let config = ServerConfig::default();
    assert_eq!(config.max_multipart_field_mb(), 100);
}

#[test]
fn test_max_bytes_to_mb_rounding() {
    let mut config = ServerConfig {
        max_request_body_bytes: 1_048_576, // 1 MB
        ..Default::default()
    };
    assert_eq!(config.max_request_body_mb(), 1);

    config.max_request_body_bytes = 1_048_577; // 1 MB + 1 byte
    assert_eq!(config.max_request_body_mb(), 2); // Rounds up
}

#[test]
fn test_serde_default_serialization() {
    let config = ServerConfig::default();
    let json = serde_json::to_string(&config).unwrap();

    assert!(json.contains("host"));
    assert!(json.contains("port"));
}
