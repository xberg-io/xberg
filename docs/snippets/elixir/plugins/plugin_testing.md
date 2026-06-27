<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
Plugin testing in Elixir is limited since custom plugins cannot be implemented in Elixir. Plugin testing should be done in Rust using `#[cfg(test)]` test modules.

When testing custom plugins in Rust:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_custom_extractor() {
        let extractor = MyExtractor;
        let content = b"test content";
        let result = extractor.extract(content, "text/plain", &ExtractionConfig::default()).await;
        assert!(result.is_ok());
    }
}
```

For Elixir, you can test extraction results with `Xberg.extract/2` and `Xberg.extract_batch/2`.
