//! Tests for environment variable overrides.

#![allow(unsafe_code)]

use crate::core::ServerConfig;

#[serial_test::serial]
#[test]
fn test_apply_env_host_override() {
    let original = std::env::var("XBERG_HOST").ok();
    unsafe {
        std::env::set_var("XBERG_HOST", "192.168.1.1");
    }

    let mut config = ServerConfig::default();
    config.apply_env_overrides().unwrap();

    assert_eq!(config.host, "192.168.1.1");

    unsafe {
        if let Some(orig) = original {
            std::env::set_var("XBERG_HOST", orig);
        } else {
            std::env::remove_var("XBERG_HOST");
        }
    }
}

#[serial_test::serial]
#[test]
fn test_apply_env_port_override() {
    let original = std::env::var("XBERG_PORT").ok();
    unsafe {
        std::env::set_var("XBERG_PORT", "5000");
    }

    let mut config = ServerConfig::default();
    config.apply_env_overrides().unwrap();

    assert_eq!(config.port, 5000);

    unsafe {
        if let Some(orig) = original {
            std::env::set_var("XBERG_PORT", orig);
        } else {
            std::env::remove_var("XBERG_PORT");
        }
    }
}

#[serial_test::serial]
#[test]
fn test_apply_env_port_invalid() {
    let original = std::env::var("XBERG_PORT").ok();
    unsafe {
        std::env::set_var("XBERG_PORT", "not_a_number");
    }

    let mut config = ServerConfig::default();
    let result = config.apply_env_overrides();

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("XBERG_PORT must be a valid u16")
    );

    unsafe {
        if let Some(orig) = original {
            std::env::set_var("XBERG_PORT", orig);
        } else {
            std::env::remove_var("XBERG_PORT");
        }
    }
}

#[serial_test::serial]
#[test]
fn test_apply_env_cors_origins_override() {
    let original = std::env::var("XBERG_CORS_ORIGINS").ok();
    unsafe {
        std::env::set_var("XBERG_CORS_ORIGINS", "https://example.com, https://other.com");
    }

    let mut config = ServerConfig::default();
    config.apply_env_overrides().unwrap();

    assert_eq!(config.cors_origins.len(), 2);
    assert!(config.cors_origins.contains(&"https://example.com".to_string()));
    assert!(config.cors_origins.contains(&"https://other.com".to_string()));

    unsafe {
        if let Some(orig) = original {
            std::env::set_var("XBERG_CORS_ORIGINS", orig);
        } else {
            std::env::remove_var("XBERG_CORS_ORIGINS");
        }
    }
}

#[serial_test::serial]
#[test]
fn test_apply_env_max_request_body_bytes_override() {
    let original = std::env::var("XBERG_MAX_REQUEST_BODY_BYTES").ok();
    unsafe {
        std::env::set_var("XBERG_MAX_REQUEST_BODY_BYTES", "52428800");
    }

    let mut config = ServerConfig::default();
    config.apply_env_overrides().unwrap();

    assert_eq!(config.max_request_body_bytes, 52_428_800);

    unsafe {
        if let Some(orig) = original {
            std::env::set_var("XBERG_MAX_REQUEST_BODY_BYTES", orig);
        } else {
            std::env::remove_var("XBERG_MAX_REQUEST_BODY_BYTES");
        }
    }
}

#[serial_test::serial]
#[test]
fn test_apply_env_max_multipart_field_bytes_override() {
    let original = std::env::var("XBERG_MAX_MULTIPART_FIELD_BYTES").ok();
    unsafe {
        std::env::set_var("XBERG_MAX_MULTIPART_FIELD_BYTES", "78643200");
    }

    let mut config = ServerConfig::default();
    config.apply_env_overrides().unwrap();

    assert_eq!(config.max_multipart_field_bytes, 78_643_200);

    unsafe {
        if let Some(orig) = original {
            std::env::set_var("XBERG_MAX_MULTIPART_FIELD_BYTES", orig);
        } else {
            std::env::remove_var("XBERG_MAX_MULTIPART_FIELD_BYTES");
        }
    }
}

#[serial_test::serial]
#[test]
fn test_apply_env_job_timeout_secs_override() {
    let original = std::env::var("XBERG_JOB_TIMEOUT_SECS").ok();
    unsafe {
        std::env::set_var("XBERG_JOB_TIMEOUT_SECS", "120");
    }

    let mut config = ServerConfig::default();
    config.apply_env_overrides().unwrap();

    assert_eq!(config.job_timeout_secs, 120);

    unsafe {
        if let Some(orig) = original {
            std::env::set_var("XBERG_JOB_TIMEOUT_SECS", orig);
        } else {
            std::env::remove_var("XBERG_JOB_TIMEOUT_SECS");
        }
    }
}

#[serial_test::serial]
#[test]
fn test_apply_env_job_timeout_secs_invalid() {
    let original = std::env::var("XBERG_JOB_TIMEOUT_SECS").ok();
    unsafe {
        std::env::set_var("XBERG_JOB_TIMEOUT_SECS", "not_a_number");
    }

    let mut config = ServerConfig::default();
    let result = config.apply_env_overrides();

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("XBERG_JOB_TIMEOUT_SECS must be a valid u64")
    );

    unsafe {
        if let Some(orig) = original {
            std::env::set_var("XBERG_JOB_TIMEOUT_SECS", orig);
        } else {
            std::env::remove_var("XBERG_JOB_TIMEOUT_SECS");
        }
    }
}

#[serial_test::serial]
#[test]
fn test_apply_env_multiple_overrides() {
    let host_orig = std::env::var("XBERG_HOST").ok();
    let port_orig = std::env::var("XBERG_PORT").ok();
    let cors_orig = std::env::var("XBERG_CORS_ORIGINS").ok();

    unsafe {
        std::env::set_var("XBERG_HOST", "0.0.0.0");
        std::env::set_var("XBERG_PORT", "4000");
        std::env::set_var("XBERG_CORS_ORIGINS", "https://api.example.com");
    }

    let mut config = ServerConfig::default();
    config.apply_env_overrides().unwrap();

    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 4000);
    assert_eq!(config.cors_origins.len(), 1);
    assert_eq!(config.cors_origins[0], "https://api.example.com");

    unsafe {
        if let Some(orig) = host_orig {
            std::env::set_var("XBERG_HOST", orig);
        } else {
            std::env::remove_var("XBERG_HOST");
        }
        if let Some(orig) = port_orig {
            std::env::set_var("XBERG_PORT", orig);
        } else {
            std::env::remove_var("XBERG_PORT");
        }
        if let Some(orig) = cors_orig {
            std::env::set_var("XBERG_CORS_ORIGINS", orig);
        } else {
            std::env::remove_var("XBERG_CORS_ORIGINS");
        }
    }
}
