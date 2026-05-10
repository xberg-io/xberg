<!-- snippet:skip -->

Custom post-processor implementation is not available in the Elixir binding. Post-processors must be implemented in Rust using the `PostProcessor` trait.

To implement a PDF-only post-processor in Rust:

```rust
use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
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
        result: &mut ExtractionResult,
        _config: &ExtractionConfig
    ) -> Result<()> {
        // Custom processing logic for PDF documents
        Ok(())
    }

    fn should_process(
        &self,
        result: &ExtractionResult,
        _config: &ExtractionConfig
    ) -> bool {
        result.mime_type == "application/pdf"
    }
}
```

Register this in your Rust initialization and Elixir will use it automatically during extraction.
