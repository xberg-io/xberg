//! OCR backend registry.

use crate::plugins::OcrBackend;
use crate::{KreuzbergError, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for OCR backend plugins.
///
/// Manages OCR backends with backend type and language-based selection.
///
/// # Thread Safety
///
/// The registry is thread-safe and can be accessed concurrently from multiple threads.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::plugins::registry::OcrBackendRegistry;
/// use std::sync::Arc;
///
/// let registry = OcrBackendRegistry::new();
/// // Register OCR backends
/// // registry.register(Arc::new(TesseractBackend::new()));
/// ```
pub struct OcrBackendRegistry {
    pub(super) backends: HashMap<String, Arc<dyn OcrBackend>>,
}

impl OcrBackendRegistry {
    /// Create a new OCR backend registry with default backends.
    ///
    /// Registers the Tesseract backend by default if the "ocr" feature is enabled.
    /// Logs warnings if backend initialization fails (common in containerized environments
    /// with missing dependencies or permission issues).
    pub fn new() -> Self {
        #[cfg(feature = "ocr")]
        let mut registry = Self {
            backends: HashMap::new(),
        };

        #[cfg(not(feature = "ocr"))]
        let registry = Self {
            backends: HashMap::new(),
        };

        #[cfg(feature = "ocr")]
        {
            use crate::ocr::tesseract_backend::TesseractBackend;
            match TesseractBackend::new() {
                Ok(backend) => {
                    if let Err(e) = registry.register(Arc::new(backend)) {
                        tracing::error!(
                            "Failed to register Tesseract OCR backend: {}. \
                             OCR functionality will be unavailable. \
                             Check TESSDATA_PREFIX environment variable and tessdata file permissions.",
                            e
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Tesseract OCR backend initialization failed: {}. \
                         OCR functionality will be unavailable. \
                         Common causes: missing TESSDATA_PREFIX env var, \
                         tessdata files not found, or permission issues in containerized environments. \
                         See https://docs.kreuzberg.dev/guides/docker/ for Kubernetes troubleshooting.",
                        e
                    );
                }
            }
        }

        registry
    }

    /// Create a new empty OCR backend registry without default backends.
    ///
    /// This is useful for testing or when you want full control over backend registration.
    pub fn new_empty() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    /// Register an OCR backend.
    ///
    /// # Arguments
    ///
    /// * `backend` - The OCR backend to register
    ///
    /// # Returns
    ///
    /// - `Ok(())` if registration succeeded
    /// - `Err(...)` if initialization failed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kreuzberg::plugins::registry::OcrBackendRegistry;
    /// # use std::sync::Arc;
    /// let mut registry = OcrBackendRegistry::new();
    /// // let backend = Arc::new(MyOcrBackend::new());
    /// // registry.register(backend)?;
    /// # Ok::<(), kreuzberg::KreuzbergError>(())
    /// ```
    pub fn register(&mut self, backend: Arc<dyn OcrBackend>) -> Result<()> {
        let name = backend.name().to_string();

        super::validate_plugin_name(&name)?;

        backend.initialize()?;

        self.backends.insert(name, backend);
        Ok(())
    }

    /// Get an OCR backend by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Backend name
    ///
    /// # Returns
    ///
    /// The backend if found, or an error if not registered.
    pub fn get(&self, name: &str) -> Result<Arc<dyn OcrBackend>> {
        self.backends.get(name).cloned().ok_or_else(|| KreuzbergError::Plugin {
            message: format!("OCR backend '{}' not registered", name),
            plugin_name: name.to_string(),
        })
    }

    /// Get an OCR backend that supports a specific language.
    ///
    /// Returns the first backend that supports the language.
    ///
    /// # Arguments
    ///
    /// * `language` - Language code (e.g., "eng", "deu")
    ///
    /// # Returns
    ///
    /// The first backend supporting the language, or an error if none found.
    pub fn get_for_language(&self, language: &str) -> Result<Arc<dyn OcrBackend>> {
        self.backends
            .values()
            .find(|backend| backend.supports_language(language))
            .cloned()
            .ok_or_else(|| KreuzbergError::Plugin {
                message: format!("No OCR backend supports language '{}'", language),
                plugin_name: language.to_string(),
            })
    }

    /// List all registered backend names.
    pub fn list(&self) -> Vec<String> {
        self.backends.keys().cloned().collect()
    }

    /// Remove a backend from the registry.
    ///
    /// Calls `shutdown()` on the backend before removing.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        if let Some(backend) = self.backends.remove(name) {
            backend.shutdown()?;
        }
        Ok(())
    }

    /// Shutdown all backends and clear the registry.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names: Vec<_> = self.backends.keys().cloned().collect();
        for name in names {
            self.remove(&name)?;
        }
        Ok(())
    }
}

impl Default for OcrBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::OcrConfig;
    use crate::plugins::{OcrBackend, Plugin};
    use crate::types::ExtractionResult;
    use async_trait::async_trait;

    struct MockOcrBackend {
        name: String,
        languages: Vec<String>,
    }

    impl Plugin for MockOcrBackend {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl OcrBackend for MockOcrBackend {
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractionResult> {
            Ok(ExtractionResult {
                content: "test".to_string(),
                mime_type: "text/plain".to_string(),
                metadata: crate::types::Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
                images: None,
                djot_content: None,
                pages: None,
                elements: None,
            })
        }

        fn supports_language(&self, lang: &str) -> bool {
            self.languages.iter().any(|l| l == lang)
        }

        fn backend_type(&self) -> crate::plugins::ocr::OcrBackendType {
            crate::plugins::ocr::OcrBackendType::Custom
        }
    }

    #[test]
    fn test_ocr_backend_registry() {
        let mut registry = OcrBackendRegistry::new_empty();

        let backend = Arc::new(MockOcrBackend {
            name: "test-ocr".to_string(),
            languages: vec!["eng".to_string(), "deu".to_string()],
        });

        registry.register(backend).unwrap();

        let retrieved = registry.get("test-ocr").unwrap();
        assert_eq!(retrieved.name(), "test-ocr");

        let eng_backend = registry.get_for_language("eng").unwrap();
        assert_eq!(eng_backend.name(), "test-ocr");

        let names = registry.list();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"test-ocr".to_string()));
    }

    #[test]
    fn test_ocr_backend_registry_new_empty() {
        let registry = OcrBackendRegistry::new_empty();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_ocr_backend_get_missing() {
        let registry = OcrBackendRegistry::new_empty();
        let result = registry.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_ocr_backend_get_for_language_missing() {
        let registry = OcrBackendRegistry::new_empty();
        let result = registry.get_for_language("fra");
        assert!(result.is_err());
    }

    #[test]
    fn test_ocr_backend_remove() {
        let mut registry = OcrBackendRegistry::new_empty();
        let backend = Arc::new(MockOcrBackend {
            name: "test-backend".to_string(),
            languages: vec!["eng".to_string()],
        });
        registry.register(backend).unwrap();

        registry.remove("test-backend").unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_ocr_backend_shutdown_all() {
        let mut registry = OcrBackendRegistry::new_empty();
        let backend1 = Arc::new(MockOcrBackend {
            name: "backend1".to_string(),
            languages: vec!["eng".to_string()],
        });
        let backend2 = Arc::new(MockOcrBackend {
            name: "backend2".to_string(),
            languages: vec!["deu".to_string()],
        });

        registry.register(backend1).unwrap();
        registry.register(backend2).unwrap();

        registry.shutdown_all().unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    struct FailingOcrBackend {
        name: String,
        fail_on_init: bool,
    }

    impl Plugin for FailingOcrBackend {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            if self.fail_on_init {
                Err(KreuzbergError::Plugin {
                    message: "Backend initialization failed".to_string(),
                    plugin_name: self.name.clone(),
                })
            } else {
                Ok(())
            }
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl OcrBackend for FailingOcrBackend {
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> Result<ExtractionResult> {
            Ok(ExtractionResult {
                content: "test".to_string(),
                mime_type: "text/plain".to_string(),
                metadata: crate::types::Metadata::default(),
                tables: vec![],
                detected_languages: None,
                chunks: None,
                images: None,
                djot_content: None,
                pages: None,
                elements: None,
            })
        }

        fn supports_language(&self, _lang: &str) -> bool {
            false
        }

        fn backend_type(&self) -> crate::plugins::ocr::OcrBackendType {
            crate::plugins::ocr::OcrBackendType::Custom
        }
    }

    #[test]
    fn test_ocr_backend_initialization_failure_logs_error() {
        let mut registry = OcrBackendRegistry::new_empty();

        let backend = Arc::new(FailingOcrBackend {
            name: "failing-ocr".to_string(),
            fail_on_init: true,
        });

        let result = registry.register(backend);
        assert!(result.is_err());
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_ocr_backend_invalid_name_empty_logs_warning() {
        let mut registry = OcrBackendRegistry::new_empty();

        let backend = Arc::new(MockOcrBackend {
            name: "".to_string(),
            languages: vec!["eng".to_string()],
        });

        let result = registry.register(backend);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_ocr_backend_invalid_name_with_spaces_logs_warning() {
        let mut registry = OcrBackendRegistry::new_empty();

        let backend = Arc::new(MockOcrBackend {
            name: "invalid ocr backend".to_string(),
            languages: vec!["eng".to_string()],
        });

        let result = registry.register(backend);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_ocr_backend_successful_registration_logs_debug() {
        let mut registry = OcrBackendRegistry::new_empty();

        let backend = Arc::new(MockOcrBackend {
            name: "valid-ocr".to_string(),
            languages: vec!["eng".to_string()],
        });

        let result = registry.register(backend);
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_ocr_backend_multiple_registrations() {
        let mut registry = OcrBackendRegistry::new_empty();

        let backend1 = Arc::new(MockOcrBackend {
            name: "ocr-backend-1".to_string(),
            languages: vec!["eng".to_string()],
        });

        let backend2 = Arc::new(MockOcrBackend {
            name: "ocr-backend-2".to_string(),
            languages: vec!["deu".to_string()],
        });

        registry.register(backend1).unwrap();
        registry.register(backend2).unwrap();

        assert_eq!(registry.list().len(), 2);
    }
}
