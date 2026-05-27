<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
Custom PDF metadata extractor implementation is not available in the Elixir binding. Document extractors must be implemented in Rust using the `DocumentExtractor` trait.

To implement a custom PDF metadata extractor in Rust:

1. Implement the `Plugin` and `DocumentExtractor` traits
2. Add support for PDF MIME types: `application/pdf`
3. Use a PDF library (e.g., pdfium-render, pdf crate) to extract metadata
4. Register the extractor in the Rust core

See the Rust plugin documentation for implementing custom `DocumentExtractor` plugins for PDF files.
