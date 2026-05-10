<!-- snippet:skip -->

Plugin unregistration is not available in the Elixir binding. Plugin unregistration must be done in Rust using the registry APIs.

To unregister a specific plugin in Rust:

```rust
use kreuzberg::plugins::registry::get_document_extractor_registry;

let registry = get_document_extractor_registry();
registry.remove("custom-json-extractor")?;
```

In Elixir, you can only clear all plugins of a specific type using:

- `Kreuzberg.clear_document_extractors()`
- `Kreuzberg.clear_post_processors()`
- `Kreuzberg.clear_ocr_backends()`
- `Kreuzberg.clear_validators()`

To remove a single plugin, you must do so from the Rust core before Elixir starts using it.
