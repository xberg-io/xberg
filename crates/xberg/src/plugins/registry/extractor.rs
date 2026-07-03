//! Document extractor registry implementation.

use crate::core::config::{ExtractInput, ExtractionConfig};
use crate::plugins::{DocumentExtractor, InternalDocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::{Result, XbergError};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

/// Registered document extractor plus optional native pipeline capability.
#[derive(Clone)]
pub(crate) struct RegisteredDocumentExtractor {
    extractor: Arc<dyn DocumentExtractor>,
    internal: Option<Arc<dyn InternalDocumentExtractor>>,
}

impl RegisteredDocumentExtractor {
    fn public(extractor: Arc<dyn DocumentExtractor>) -> Self {
        Self {
            extractor,
            internal: None,
        }
    }

    fn internal<T>(extractor: Arc<T>) -> Self
    where
        T: InternalDocumentExtractor + 'static,
    {
        Self {
            extractor: extractor.clone(),
            internal: Some(extractor),
        }
    }

    pub(crate) fn extractor(&self) -> Arc<dyn DocumentExtractor> {
        Arc::clone(&self.extractor)
    }

    pub(crate) fn plugin(&self) -> &dyn DocumentExtractor {
        self.extractor.as_ref()
    }

    async fn extract_content_inner(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        if let Some(internal) = &self.internal {
            return internal.extract_content(content, mime_type, config).await;
        }

        let result = self
            .extractor
            .extract(
                ExtractInput::from_bytes(content.to_vec(), mime_type.to_string(), None),
                config,
            )
            .await?;
        Ok(result.into())
    }

    async fn extract_path_inner(
        &self,
        path: &Path,
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        if let Some(internal) = &self.internal {
            return internal.extract_path(path, mime_type, config).await;
        }

        let mut input = ExtractInput::from_uri(path.to_string_lossy().into_owned());
        input.mime_type = Some(mime_type.to_string());
        let result = self.extractor.extract(input, config).await?;
        Ok(result.into())
    }
}

impl Plugin for RegisteredDocumentExtractor {
    fn name(&self) -> &str {
        self.plugin().name()
    }

    fn version(&self) -> String {
        self.plugin().version()
    }

    fn initialize(&self) -> Result<()> {
        self.plugin().initialize()
    }

    fn shutdown(&self) -> Result<()> {
        self.plugin().shutdown()
    }

    fn description(&self) -> &str {
        self.plugin().description()
    }

    fn author(&self) -> &str {
        self.plugin().author()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl InternalDocumentExtractor for RegisteredDocumentExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        self.extract_content_inner(content, mime_type, config).await
    }

    async fn extract_path(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<InternalDocument> {
        self.extract_path_inner(path, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        self.plugin().supported_mime_types()
    }

    fn priority(&self) -> i32 {
        self.plugin().priority()
    }

    fn can_handle(&self, path: &Path, mime_type: &str) -> bool {
        self.plugin().can_handle(path, mime_type)
    }
}

#[cfg_attr(alef, alef(skip))]
/// Registry for document extractor plugins.
///
/// Manages extractors with MIME type and priority-based selection.
///
/// # Thread Safety
///
/// The registry is thread-safe and can be accessed concurrently from multiple threads.
pub struct DocumentExtractorRegistry {
    extractors: HashMap<String, BTreeMap<i32, RegisteredDocumentExtractor>>,
    name_index: HashMap<String, Vec<(String, i32)>>,
}

impl DocumentExtractorRegistry {
    /// Create a new empty extractor registry.
    pub fn new() -> Self {
        Self {
            extractors: HashMap::new(),
            name_index: HashMap::new(),
        }
    }

    /// Register a document extractor.
    ///
    /// The extractor is registered for all MIME types it supports.
    ///
    /// # Arguments
    ///
    /// * `extractor` - The extractor to register
    ///
    /// # Returns
    ///
    /// - `Ok(())` if registration succeeded
    /// - `Err(...)` if initialization failed
    pub fn register(&mut self, extractor: Arc<dyn DocumentExtractor>) -> Result<()> {
        self.register_entry(RegisteredDocumentExtractor::public(extractor))
    }

    /// Register a native extractor with access to the internal pipeline representation.
    pub(crate) fn register_internal<T>(&mut self, extractor: Arc<T>) -> Result<()>
    where
        T: InternalDocumentExtractor + 'static,
    {
        self.register_entry(RegisteredDocumentExtractor::internal(extractor))
    }

    fn register_entry(&mut self, entry: RegisteredDocumentExtractor) -> Result<()> {
        let extractor = entry.extractor();
        let name = extractor.name().to_string();
        let priority = extractor.priority();
        let mime_types: Vec<String> = extractor.supported_mime_types().iter().map(|s| s.to_string()).collect();

        if let Err(e) = super::validate_plugin_name(&name) {
            tracing::warn!(
                "Failed to validate document extractor name '{}': {}. \
                 Registration aborted. Plugin names must be non-empty and contain only alphanumeric characters, hyphens, and underscores.",
                name,
                e
            );
            return Err(e);
        }

        if let Err(e) = extractor.initialize() {
            tracing::error!(
                "Failed to initialize document extractor '{}': {}. \
                 Extraction for MIME types {:?} will be unavailable.",
                name,
                e,
                mime_types
            );
            return Err(e);
        }

        let mut index_entries = Vec::new();

        for mime_type in &mime_types {
            self.extractors
                .entry(mime_type.clone())
                .or_default()
                .insert(priority, entry.clone());
            index_entries.push((mime_type.clone(), priority));
        }

        self.name_index.insert(name.clone(), index_entries);
        tracing::debug!(
            "Registered document extractor '{}' with priority {} for MIME types: {:?}",
            name,
            priority,
            mime_types
        );

        Ok(())
    }

    /// Get the highest priority extractor for a MIME type.
    ///
    /// # Arguments
    ///
    /// * `mime_type` - MIME type to look up
    ///
    /// # Returns
    ///
    /// The highest priority extractor, or an error if none found.
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self),
        fields(
            registry.mime_type = %mime_type,
            registry.found = tracing::field::Empty,
        )
    ))]
    pub fn get(&self, mime_type: &str) -> Result<Arc<dyn DocumentExtractor>> {
        Ok(self.get_registered(mime_type)?.extractor())
    }

    pub(crate) fn get_registered(&self, mime_type: &str) -> Result<RegisteredDocumentExtractor> {
        if let Some(priority_map) = self.extractors.get(mime_type)
            && let Some((_priority, entry)) = priority_map.iter().next_back()
        {
            #[cfg(feature = "otel")]
            tracing::Span::current().record("registry.found", true);
            return Ok(entry.clone());
        }

        let mut best_match: Option<(i32, RegisteredDocumentExtractor)> = None;

        for (registered_mime, priority_map) in &self.extractors {
            if registered_mime.ends_with("/*") {
                let prefix = &registered_mime[..registered_mime.len() - 1];
                if mime_type.starts_with(prefix)
                    && let Some((_priority, entry)) = priority_map.iter().next_back()
                {
                    let priority = entry.extractor.priority();
                    match &best_match {
                        None => best_match = Some((priority, entry.clone())),
                        Some((current_priority, _)) => {
                            if priority > *current_priority {
                                best_match = Some((priority, entry.clone()));
                            }
                        }
                    }
                }
            }
        }

        if let Some((_priority, entry)) = best_match {
            #[cfg(feature = "otel")]
            tracing::Span::current().record("registry.found", true);
            return Ok(entry);
        }

        #[cfg(feature = "otel")]
        tracing::Span::current().record("registry.found", false);
        Err(XbergError::UnsupportedFormat(mime_type.to_string()))
    }

    /// List all registered extractors.
    pub fn list(&self) -> Vec<String> {
        self.name_index.keys().cloned().collect()
    }

    /// Remove an extractor from the registry.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let index_entries = match self.name_index.remove(name) {
            Some(entries) => entries,
            None => {
                tracing::debug!(
                    "Document extractor '{}' not found in registry (already removed or never registered)",
                    name
                );
                return Ok(());
            }
        };

        let mut extractor_to_shutdown: Option<Arc<dyn DocumentExtractor>> = None;

        for (mime_type, priority) in index_entries {
            if let Some(priority_map) = self.extractors.get_mut(&mime_type) {
                if let Some(entry) = priority_map.remove(&priority)
                    && extractor_to_shutdown.is_none()
                {
                    extractor_to_shutdown = Some(entry.extractor());
                }

                if priority_map.is_empty() {
                    self.extractors.remove(&mime_type);
                }
            }
        }

        if let Some(extractor) = extractor_to_shutdown {
            if let Err(e) = extractor.shutdown() {
                tracing::warn!(
                    "Failed to shutdown document extractor '{}': {}. \
                     Resources may not have been properly released.",
                    name,
                    e
                );
                return Err(e);
            }
            tracing::debug!("Successfully removed and shut down document extractor '{}'", name);
        }

        Ok(())
    }

    /// Shutdown all extractors and clear the registry.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names = self.list();
        let count = names.len();

        if count > 0 {
            tracing::debug!("Shutting down {} document extractors", count);
        }

        for name in names {
            self.remove(&name)?;
        }

        if count > 0 {
            tracing::debug!("Successfully shut down all {} document extractors", count);
        }
        Ok(())
    }

    /// Drain the registry. Alias for `shutdown_all` used by alef trait-bridge codegen.
    pub fn clear(&mut self) -> Result<()> {
        self.shutdown_all()
    }
}

impl Default for DocumentExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ExtractionConfig;
    use crate::plugins::{InternalDocumentExtractor, Plugin};

    use async_trait::async_trait;

    struct MockExtractor {
        name: String,
        mime_types: &'static [&'static str],
        priority: i32,
    }

    impl Plugin for MockExtractor {
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
    impl InternalDocumentExtractor for MockExtractor {
        async fn extract_content(
            &self,
            _: &[u8],
            _: &str,
            _: &ExtractionConfig,
        ) -> Result<crate::types::internal::InternalDocument> {
            Ok(crate::types::internal::InternalDocument::new("mock"))
        }

        fn supported_mime_types(&self) -> &[&str] {
            self.mime_types
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[test]
    fn test_document_extractor_registry_exact_match() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "pdf-extractor".to_string(),
            mime_types: &["application/pdf"],
            priority: 100,
        });

        registry.register(extractor).unwrap();

        let retrieved = registry.get("application/pdf").unwrap();
        assert_eq!(retrieved.name(), "pdf-extractor");

        let names = registry.list();
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"pdf-extractor".to_string()));
    }

    #[test]
    fn test_document_extractor_registry_prefix_match() {
        let mut registry = DocumentExtractorRegistry::new();

        let image_extractor = Arc::new(MockExtractor {
            name: "image-extractor".to_string(),
            mime_types: &["image/*"],
            priority: 50,
        });

        registry.register(image_extractor).unwrap();

        let retrieved = registry.get("image/png").unwrap();
        assert_eq!(retrieved.name(), "image-extractor");

        let retrieved_jpg = registry.get("image/jpeg").unwrap();
        assert_eq!(retrieved_jpg.name(), "image-extractor");
    }

    #[test]
    fn test_document_extractor_registry_priority() {
        let mut registry = DocumentExtractorRegistry::new();

        let low_priority = Arc::new(MockExtractor {
            name: "low-priority-pdf".to_string(),
            mime_types: &["application/pdf"],
            priority: 10,
        });

        let high_priority = Arc::new(MockExtractor {
            name: "high-priority-pdf".to_string(),
            mime_types: &["application/pdf"],
            priority: 100,
        });

        registry.register(low_priority).unwrap();
        registry.register(high_priority).unwrap();

        let retrieved = registry.get("application/pdf").unwrap();
        assert_eq!(retrieved.name(), "high-priority-pdf");
    }

    #[test]
    fn test_document_extractor_registry_not_found() {
        let registry = DocumentExtractorRegistry::new();

        let result = registry.get("application/unknown");
        assert!(matches!(result, Err(XbergError::UnsupportedFormat(_))));
    }

    #[test]
    fn test_document_extractor_registry_remove() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "test-extractor".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        registry.register(extractor).unwrap();
        assert!(registry.get("text/plain").is_ok());

        registry.remove("test-extractor").unwrap();
        assert!(registry.get("text/plain").is_err());
    }

    #[test]
    fn test_document_extractor_registry_shutdown_all() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor1 = Arc::new(MockExtractor {
            name: "extractor1".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let extractor2 = Arc::new(MockExtractor {
            name: "extractor2".to_string(),
            mime_types: &["application/pdf"],
            priority: 50,
        });

        registry.register(extractor1).unwrap();
        registry.register(extractor2).unwrap();

        assert_eq!(registry.list().len(), 2);

        registry.shutdown_all().unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_document_extractor_registry_default() {
        let registry = DocumentExtractorRegistry::default();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_document_extractor_registry_exact_over_prefix() {
        let mut registry = DocumentExtractorRegistry::new();

        let prefix_extractor = Arc::new(MockExtractor {
            name: "prefix-extractor".to_string(),
            mime_types: &["image/*"],
            priority: 100,
        });

        let exact_extractor = Arc::new(MockExtractor {
            name: "exact-extractor".to_string(),
            mime_types: &["image/png"],
            priority: 50,
        });

        registry.register(prefix_extractor).unwrap();
        registry.register(exact_extractor).unwrap();

        let retrieved = registry.get("image/png").unwrap();
        assert_eq!(retrieved.name(), "exact-extractor");

        let retrieved_jpg = registry.get("image/jpeg").unwrap();
        assert_eq!(retrieved_jpg.name(), "prefix-extractor");
    }

    #[test]
    fn test_document_extractor_registry_invalid_name_empty() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let result = registry.register(extractor);
        assert!(matches!(result, Err(XbergError::Validation { .. })));
    }

    #[test]
    fn test_document_extractor_registry_invalid_name_whitespace() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "my extractor".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let result = registry.register(extractor);
        assert!(matches!(result, Err(XbergError::Validation { .. })));
    }

    #[test]
    fn test_document_extractor_registry_multiple_mime_types() {
        let mut registry = DocumentExtractorRegistry::new();

        let multi_extractor = Arc::new(MockExtractor {
            name: "multi-extractor".to_string(),
            mime_types: &["text/plain", "text/markdown", "text/html"],
            priority: 50,
        });

        registry.register(multi_extractor).unwrap();

        assert_eq!(registry.get("text/plain").unwrap().name(), "multi-extractor");
        assert_eq!(registry.get("text/markdown").unwrap().name(), "multi-extractor");
        assert_eq!(registry.get("text/html").unwrap().name(), "multi-extractor");
    }

    struct FailingExtractor {
        name: String,
        fail_on_init: bool,
    }

    impl Plugin for FailingExtractor {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            if self.fail_on_init {
                Err(XbergError::Plugin {
                    message: "Extractor initialization failed".to_string(),
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
    impl InternalDocumentExtractor for FailingExtractor {
        async fn extract_content(
            &self,
            _: &[u8],
            _: &str,
            _: &ExtractionConfig,
        ) -> Result<crate::types::internal::InternalDocument> {
            Ok(crate::types::internal::InternalDocument::new("mock"))
        }

        fn supported_mime_types(&self) -> &[&str] {
            &["text/plain"]
        }

        fn priority(&self) -> i32 {
            50
        }
    }

    #[test]
    fn test_document_extractor_initialization_failure_logs_error() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(FailingExtractor {
            name: "failing-extractor".to_string(),
            fail_on_init: true,
        });

        let result = registry.register(extractor);
        assert!(result.is_err());
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_document_extractor_invalid_name_empty_logs_warning() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let result = registry.register(extractor);
        assert!(matches!(result, Err(XbergError::Validation { .. })));
    }

    #[test]
    fn test_document_extractor_invalid_name_with_spaces_logs_warning() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "invalid extractor".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let result = registry.register(extractor);
        assert!(matches!(result, Err(XbergError::Validation { .. })));
    }

    #[test]
    fn test_document_extractor_successful_registration_logs_debug() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor = Arc::new(MockExtractor {
            name: "valid-pdf-extractor".to_string(),
            mime_types: &["application/pdf"],
            priority: 100,
        });

        let result = registry.register(extractor);
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_document_extractor_remove_nonexistent_logs_debug() {
        let mut registry = DocumentExtractorRegistry::new();

        let result = registry.remove("nonexistent-extractor");
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_document_extractor_shutdown_empty_registry() {
        let mut registry = DocumentExtractorRegistry::new();
        let result = registry.shutdown_all();
        assert!(result.is_ok());
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_document_extractor_shutdown_with_multiple_extractors() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractor1 = Arc::new(MockExtractor {
            name: "extractor1".to_string(),
            mime_types: &["text/plain"],
            priority: 50,
        });

        let extractor2 = Arc::new(MockExtractor {
            name: "extractor2".to_string(),
            mime_types: &["application/pdf"],
            priority: 100,
        });

        let extractor3 = Arc::new(MockExtractor {
            name: "extractor3".to_string(),
            mime_types: &["image/png"],
            priority: 75,
        });

        registry.register(extractor1).unwrap();
        registry.register(extractor2).unwrap();
        registry.register(extractor3).unwrap();

        assert_eq!(registry.list().len(), 3);

        registry.shutdown_all().unwrap();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_document_extractor_priority_ordering_complex() {
        let mut registry = DocumentExtractorRegistry::new();

        let extractors = vec![
            (
                Arc::new(MockExtractor {
                    name: "priority-1".to_string(),
                    mime_types: &["application/pdf"],
                    priority: 1,
                }),
                1,
            ),
            (
                Arc::new(MockExtractor {
                    name: "priority-100".to_string(),
                    mime_types: &["application/pdf"],
                    priority: 100,
                }),
                100,
            ),
            (
                Arc::new(MockExtractor {
                    name: "priority-50".to_string(),
                    mime_types: &["application/pdf"],
                    priority: 50,
                }),
                50,
            ),
        ];

        for (extractor, _priority) in &extractors {
            registry.register(extractor.clone()).unwrap();
        }

        let retrieved = registry.get("application/pdf").unwrap();
        assert_eq!(retrieved.name(), "priority-100");
    }
}
