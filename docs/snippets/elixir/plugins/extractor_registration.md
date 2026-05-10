<!-- snippet:skip -->

Document extractor registration is not available in the Elixir binding. Custom extractors must be implemented in Rust using the `DocumentExtractor` trait and registered in the Rust core.

To use custom extractors in Elixir:

1. Implement the extractor in Rust using the `DocumentExtractor` trait
2. Register the extractor in the Rust core's registry
3. Call the extraction functions from Elixir

See the Rust plugin documentation for implementing custom `DocumentExtractor` plugins.
