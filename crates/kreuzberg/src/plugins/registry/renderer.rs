//! Renderer registry.

use crate::plugins::Renderer;
use crate::types::internal::InternalDocument;
use crate::{KreuzbergError, Result};
use ahash::AHashMap;
use std::sync::Arc;

/// Built-in Markdown renderer.
struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    fn name(&self) -> &str {
        "markdown"
    }

    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_markdown(doc))
    }
}

/// Built-in HTML renderer.
struct HtmlRenderer;

impl Renderer for HtmlRenderer {
    fn name(&self) -> &str {
        "html"
    }

    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_html(doc))
    }
}

/// Built-in Djot renderer.
struct DjotRenderer;

impl Renderer for DjotRenderer {
    fn name(&self) -> &str {
        "djot"
    }

    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_djot(doc))
    }
}

/// Built-in plain text renderer.
struct PlainRenderer;

impl Renderer for PlainRenderer {
    fn name(&self) -> &str {
        "plain"
    }

    fn render(&self, doc: &InternalDocument) -> Result<String> {
        Ok(crate::rendering::render_plain(doc))
    }
}

/// Registry for document renderer plugins.
///
/// Manages renderers that convert [`InternalDocument`] to output format strings.
///
/// # Thread Safety
///
/// The registry is thread-safe and can be accessed concurrently from multiple threads.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::plugins::registry::RendererRegistry;
/// use std::sync::Arc;
///
/// let registry = RendererRegistry::new();
/// let available = registry.list();
/// // Built-in renderers: "markdown", "html", "djot", "plain"
/// ```
pub struct RendererRegistry {
    renderers: AHashMap<String, Arc<dyn Renderer>>,
}

impl RendererRegistry {
    /// Create a new renderer registry with built-in renderers.
    ///
    /// Registers the following built-in renderers:
    /// - `markdown` — GFM Markdown (via comrak)
    /// - `html` — HTML5 (via comrak)
    /// - `djot` — Djot markup
    /// - `plain` — Plain text (no formatting)
    pub fn new() -> Self {
        let mut registry = Self {
            renderers: AHashMap::new(),
        };

        registry.register_builtins();
        registry
    }

    /// Create a new empty renderer registry without built-in renderers.
    ///
    /// Useful for testing or when you want full control over renderer registration.
    pub fn new_empty() -> Self {
        Self {
            renderers: AHashMap::new(),
        }
    }

    /// Register built-in renderers.
    fn register_builtins(&mut self) {
        // Built-in renderers do not go through validate_plugin_name
        // since they are known-good names.
        self.renderers
            .insert("markdown".to_string(), Arc::new(MarkdownRenderer));
        self.renderers.insert("html".to_string(), Arc::new(HtmlRenderer));
        self.renderers.insert("djot".to_string(), Arc::new(DjotRenderer));
        self.renderers.insert("plain".to_string(), Arc::new(PlainRenderer));
    }

    /// Register a renderer.
    ///
    /// # Arguments
    ///
    /// * `renderer` - The renderer to register
    ///
    /// # Returns
    ///
    /// - `Ok(())` if registration succeeded
    /// - `Err(...)` if the renderer name is invalid
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use kreuzberg::plugins::registry::RendererRegistry;
    /// # use std::sync::Arc;
    /// let mut registry = RendererRegistry::new();
    /// // let renderer = Arc::new(MyRenderer);
    /// // registry.register(renderer)?;
    /// # Ok::<(), kreuzberg::KreuzbergError>(())
    /// ```
    pub fn register(&mut self, renderer: Arc<dyn Renderer>) -> Result<()> {
        let name = renderer.name().to_string();

        super::validate_plugin_name(&name)?;

        self.renderers.insert(name, renderer);
        Ok(())
    }

    /// Get a renderer by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Renderer name (e.g., "markdown", "html")
    ///
    /// # Returns
    ///
    /// The renderer if found, or an error if not registered.
    pub(crate) fn get(&self, name: &str) -> Result<Arc<dyn Renderer>> {
        self.renderers.get(name).cloned().ok_or_else(|| KreuzbergError::Plugin {
            message: format!("Renderer '{}' not registered", name),
            plugin_name: name.to_string(),
        })
    }

    /// Render a document using the named renderer.
    ///
    /// Convenience method that looks up the renderer by name and renders the document.
    ///
    /// # Arguments
    ///
    /// * `name` - Renderer name (e.g., "markdown", "html")
    /// * `doc` - The internal document to render
    ///
    /// # Returns
    ///
    /// The rendered output string, or an error if the renderer is not found or rendering fails.
    pub(crate) fn render(&self, name: &str, doc: &InternalDocument) -> Result<String> {
        let renderer = self.get(name)?;
        renderer.render(doc)
    }

    /// List all registered renderer names.
    pub fn list(&self) -> Vec<String> {
        self.renderers.keys().cloned().collect()
    }

    /// Remove a renderer from the registry.
    pub fn remove(&mut self, name: &str) {
        self.renderers.remove(name);
    }

    /// Clear all renderers and re-register the built-in defaults.
    #[cfg(test)]
    pub(crate) fn reset_to_defaults(&mut self) -> Result<()> {
        self.renderers.clear();
        self.register_builtins();
        Ok(())
    }
}

impl Default for RendererRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRenderer {
        format_name: String,
    }

    impl Renderer for MockRenderer {
        fn name(&self) -> &str {
            &self.format_name
        }

        fn render(&self, doc: &InternalDocument) -> Result<String> {
            Ok(format!("mock-rendered-{}-elements", doc.elements.len()))
        }
    }

    #[test]
    fn test_renderer_registry_new_has_builtins() {
        let registry = RendererRegistry::new();
        let names = registry.list();
        assert!(names.contains(&"markdown".to_string()));
        assert!(names.contains(&"html".to_string()));
        assert!(names.contains(&"djot".to_string()));
        assert!(names.contains(&"plain".to_string()));
    }

    #[test]
    fn test_renderer_registry_new_empty() {
        let registry = RendererRegistry::new_empty();
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_renderer_registry_register_and_get() {
        let mut registry = RendererRegistry::new_empty();

        let renderer = Arc::new(MockRenderer {
            format_name: "test-format".to_string(),
        });

        registry.register(renderer).unwrap();

        let retrieved = registry.get("test-format").unwrap();
        assert_eq!(retrieved.name(), "test-format");
    }

    #[test]
    fn test_renderer_registry_get_missing() {
        let registry = RendererRegistry::new_empty();
        let result = registry.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_renderer_registry_render_convenience() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("plain", &doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_renderer_registry_render_missing() {
        let registry = RendererRegistry::new_empty();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("nonexistent", &doc);
        assert!(result.is_err());
    }

    #[test]
    fn test_renderer_registry_remove() {
        let mut registry = RendererRegistry::new_empty();
        let renderer = Arc::new(MockRenderer {
            format_name: "to-remove".to_string(),
        });
        registry.register(renderer).unwrap();

        registry.remove("to-remove");
        assert_eq!(registry.list().len(), 0);
    }

    #[test]
    fn test_renderer_registry_reset_to_defaults() {
        let mut registry = RendererRegistry::new();
        let custom = Arc::new(MockRenderer {
            format_name: "custom".to_string(),
        });
        registry.register(custom).unwrap();
        assert!(registry.list().contains(&"custom".to_string()));

        registry.reset_to_defaults().unwrap();
        assert!(!registry.list().contains(&"custom".to_string()));
        assert!(registry.list().contains(&"markdown".to_string()));
    }

    #[test]
    fn test_renderer_registry_invalid_name_empty() {
        let mut registry = RendererRegistry::new_empty();
        let renderer = Arc::new(MockRenderer {
            format_name: "".to_string(),
        });

        let result = registry.register(renderer);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_renderer_registry_invalid_name_with_spaces() {
        let mut registry = RendererRegistry::new_empty();
        let renderer = Arc::new(MockRenderer {
            format_name: "invalid format".to_string(),
        });

        let result = registry.register(renderer);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_renderer_registry_builtin_markdown_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("markdown", &doc).unwrap();
        // Should not panic; empty doc produces empty or minimal output
        // Verify rendering succeeds without panic
        let _ = result;
    }

    #[test]
    fn test_renderer_registry_builtin_html_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("html", &doc).unwrap();
        // Verify rendering succeeds without panic
        let _ = result;
    }

    #[test]
    fn test_renderer_registry_builtin_djot_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("djot", &doc).unwrap();
        // Verify rendering succeeds without panic
        let _ = result;
    }

    #[test]
    fn test_renderer_registry_builtin_plain_renders() {
        let registry = RendererRegistry::new();
        let doc = InternalDocument::new("text/plain");

        let result = registry.render("plain", &doc).unwrap();
        // Verify rendering succeeds without panic
        let _ = result;
    }
}
