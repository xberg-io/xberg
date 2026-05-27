<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
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
