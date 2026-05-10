//! Renderer plugin trait.
//!
//! This module defines the trait for implementing custom document renderers
//! that convert [`InternalDocument`] to output format strings.

use std::sync::Arc;

use crate::Result;
use crate::plugins::Plugin;
use crate::types::internal::InternalDocument;

/// Trait for document renderers that convert [`InternalDocument`] to output strings.
///
/// Renderers are typically stateless converters that transform the internal
/// document representation into a specific output format (Markdown, HTML,
/// Djot, plain text, etc.). They participate in the standard [`Plugin`]
/// lifecycle so custom renderers can be registered from any supported binding
/// language.
///
/// The format name is exposed via [`Plugin::name`]. For stateless renderers
/// the [`Plugin`] lifecycle methods (`version`, `initialize`, `shutdown`) all
/// take no-op defaults and need not be overridden.
///
/// # Thread Safety
///
/// Renderers must be `Send + Sync` (inherited from [`Plugin`]).
///
/// # Example
///
/// ```rust
/// use kreuzberg::plugins::{Plugin, Renderer};
/// use kreuzberg::types::internal::InternalDocument;
/// use kreuzberg::Result;
///
/// struct CustomRenderer;
///
/// impl Plugin for CustomRenderer {
///     fn name(&self) -> &str { "custom" }
/// }
///
/// impl Renderer for CustomRenderer {
///     fn render(&self, doc: &InternalDocument) -> Result<String> {
///         Ok(format!("Custom output with {} elements", doc.elements.len()))
///     }
/// }
/// ```
pub trait Renderer: Plugin {
    /// Render an [`InternalDocument`] to the output format.
    ///
    /// # Arguments
    ///
    /// * `doc` - The internal document to render
    ///
    /// # Returns
    ///
    /// The rendered output as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails.
    fn render(&self, doc: &InternalDocument) -> Result<String>;
}

/// Register a renderer plugin with the global registry.
///
/// The renderer's format name is taken from [`Plugin::name`]. Registering a
/// renderer with a name that already exists replaces the previous renderer
/// for that format.
pub fn register_renderer(renderer: Arc<dyn Renderer>) -> Result<()> {
    use crate::plugins::registry::get_renderer_registry;

    let registry = get_renderer_registry();
    let mut registry = registry.write();
    registry.register(renderer)
}

/// Unregister a renderer by format name.
///
/// # Errors
///
/// Returns an error if the registry lock is poisoned.
pub fn unregister_renderer(name: &str) -> Result<()> {
    use crate::plugins::registry::get_renderer_registry;

    let registry = get_renderer_registry();
    let mut registry = registry.write();
    registry.remove(name)
}

/// List names of all registered renderers.
///
/// # Errors
///
/// Returns an error if the registry lock is poisoned.
pub fn list_renderers() -> Result<Vec<String>> {
    use crate::plugins::registry::get_renderer_registry;

    let registry = get_renderer_registry();
    let registry = registry.read();
    Ok(registry.list())
}

/// Clear all renderers from the global registry.
///
/// Removes every renderer, including the built-in defaults (markdown, html,
/// djot, plain). After calling this no renderers are registered; re-register
/// as needed.
///
/// # Errors
///
/// Returns an error if the registry lock is poisoned.
pub fn clear_renderers() -> Result<()> {
    use crate::plugins::registry::get_renderer_registry;

    let registry = get_renderer_registry();
    let mut registry = registry.write();
    registry.clear_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRenderer {
        format: &'static str,
    }

    impl Plugin for MockRenderer {
        fn name(&self) -> &str {
            self.format
        }
    }

    impl Renderer for MockRenderer {
        fn render(&self, doc: &crate::types::internal::InternalDocument) -> crate::Result<String> {
            Ok(format!("mock-{}-{}", self.format, doc.elements.len()))
        }
    }

    #[test]
    fn register_list_unregister_roundtrip() {
        register_renderer(Arc::new(MockRenderer { format: "test-fmt-a" })).unwrap();
        assert!(list_renderers().unwrap().contains(&"test-fmt-a".to_string()));

        unregister_renderer("test-fmt-a").unwrap();
        assert!(!list_renderers().unwrap().contains(&"test-fmt-a".to_string()));
    }

    #[test]
    fn register_list_clear_list_roundtrip() {
        register_renderer(Arc::new(MockRenderer { format: "test-fmt-b" })).unwrap();
        assert!(list_renderers().unwrap().contains(&"test-fmt-b".to_string()));

        clear_renderers().unwrap();
        assert!(list_renderers().unwrap().is_empty());

        // Restore built-ins so other tests are unaffected.
        use crate::plugins::registry::get_renderer_registry;
        let registry = get_renderer_registry();
        let mut registry = registry.write();
        registry.reset_to_defaults().unwrap();
    }
}
