```rust title="Rust"
use xberg::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor};
use xberg::{Result, ExtractedDocument, ExtractionConfig};
use async_trait::async_trait;
use std::sync::Arc;
use serde_json::json;

struct PdfOnlyProcessor;

impl Plugin for PdfOnlyProcessor {
    fn name(&self) -> &str { "pdf-only-processor" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl PostProcessor for PdfOnlyProcessor {
    async fn process(
        &self,
        result: &mut ExtractedDocument,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        if result.mime_type != "application/pdf" {
            return Ok(());
        }

        result.metadata.additional.insert("pdf_processed".to_string(), json!(true));

        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }

    fn should_process(
        &self,
        result: &ExtractedDocument,
        _config: &ExtractionConfig,
    ) -> bool {
        result.mime_type == "application/pdf"
    }
}

fn main() -> Result<()> {
    register_post_processor(Arc::new(PdfOnlyProcessor))?;
    Ok(())
}
```
