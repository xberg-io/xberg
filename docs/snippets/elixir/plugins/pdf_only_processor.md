<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
Custom post-processor implementation is not available in the Elixir binding. Post-processors must be implemented in Rust using the `PostProcessor` trait.

To implement a PDF-only post-processor in Rust:

```rust
use xberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use xberg::{Result, ExtractedDocument, ExtractionConfig};
use async_trait::async_trait;

struct PdfOnlyProcessor;

impl Plugin for PdfOnlyProcessor {
    fn name(&self) -> &str { "pdf-only" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl PostProcessor for PdfOnlyProcessor {
    async fn process(
        &self,
        result: &mut ExtractedDocument,
        _config: &ExtractionConfig
    ) -> Result<()> {
        // Custom processing logic for PDF documents
        Ok(())
    }

    fn should_process(
        &self,
        result: &ExtractedDocument,
        _config: &ExtractionConfig
    ) -> bool {
        result.mime_type == "application/pdf"
    }
}
```

Register this in your Rust initialization and Elixir will use it automatically during extraction.
