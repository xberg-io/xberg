<!-- snippet:skip -->

Embedding backend registration is not available in the Elixir binding. Custom embedding backends must be implemented in Rust using the `EmbeddingBackend` trait and registered in the Rust core before being used by Elixir.

To use custom embeddings in Elixir:

1. Implement the embedding backend in Rust (in `crates/kreuzberg/src/plugins/embedding.rs` or a separate Rust crate)
2. Register the backend in the Rust core initialization
3. Call the embeddings functions from Elixir with the appropriate config

See the Rust plugin documentation for implementing custom `EmbeddingBackend` plugins.
