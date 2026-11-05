use crate::core::config::OcrConfig;
use crate::plugins::OcrBackend;
use std::sync::Arc;

/// Keep the source raster live while admitting the built-in backend's own image work.
pub(crate) fn reserve_retained_ocr_raster(
    backend: &Arc<dyn OcrBackend>,
    mut configs: (OcrConfig, OcrConfig),
    retained_bytes: usize,
) -> crate::Result<(OcrConfig, OcrConfig)> {
    #[cfg(feature = "ocr")]
    let builtin = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .is_builtin_tesseract(backend);
    #[cfg(not(feature = "ocr"))]
    let builtin = false;
    if retained_bytes > 0 && builtin {
        for config in [&mut configs.0, &mut configs.1] {
            let mut limits = config.security_limits.clone().unwrap_or_default();
            limits.max_content_size = limits
                .max_content_size
                .checked_sub(retained_bytes)
                .filter(|remaining| *remaining > 0)
                .ok_or_else(|| {
                    crate::XbergError::validation(format!(
                        "Retained OCR source raster ({retained_bytes} bytes) exhausts security_limits.max_content_size \
                     ({} bytes). Raise security_limits.max_content_size",
                        limits.max_content_size,
                    ))
                })?;
            config.security_limits = Some(limits);
        }
    }
    let _ = backend;
    Ok(configs)
}

#[cfg(all(test, feature = "ocr"))]
mod tests {
    use super::*;
    use crate::plugins::{OcrBackendType, Plugin};

    struct Replacement;
    impl Plugin for Replacement {
        fn name(&self) -> &str {
            "tesseract"
        }
        fn version(&self) -> String {
            "1.0".into()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }
    #[async_trait::async_trait]
    impl OcrBackend for Replacement {
        fn backend_type(&self) -> OcrBackendType {
            OcrBackendType::Tesseract
        }
        fn supports_language(&self, _: &str) -> bool {
            true
        }
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<crate::types::ExtractedDocument> {
            panic!("budget reservation must not execute OCR")
        }
    }

    #[test]
    fn a_custom_backend_named_tesseract_keeps_its_own_budgets() {
        let backend: Arc<dyn OcrBackend> = Arc::new(Replacement);
        let config = |limit| OcrConfig {
            security_limits: Some(crate::extractors::security::SecurityLimits {
                max_content_size: limit,
                ..Default::default()
            }),
            ..Default::default()
        };
        let (original, effective) = reserve_retained_ocr_raster(&backend, (config(32), config(64)), usize::MAX)
            .expect("a matching name and backend type do not grant built-in identity");
        assert_eq!(original.security_limits.unwrap().max_content_size, 32);
        assert_eq!(effective.security_limits.unwrap().max_content_size, 64);
    }
}
