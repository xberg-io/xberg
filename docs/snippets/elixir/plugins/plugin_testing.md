<!-- snippet:skip -->

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
        let result = extractor.extract_bytes(content, "text/plain", &ExtractionConfig::default()).await;
        assert!(result.is_ok());
    }
}
```

For Elixir, you can test the extraction results using the built-in functions like `Kreuzberg.extract_bytes_async/3` and `Kreuzberg.extract_file_async/3`.
