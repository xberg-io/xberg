```rust title="Rust"
use xberg::plugins::{Plugin, PostProcessor, ProcessingStage, register_post_processor};
use xberg::{Result, ExtractedDocument, ExtractionConfig};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use serde_json::json;

struct PdfMetadataExtractor {
    processed_count: AtomicUsize,
}

impl Plugin for PdfMetadataExtractor {
    fn name(&self) -> &str { "pdf-metadata-extractor" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> {
        self.processed_count.store(0, Ordering::Release);
        Ok(())
    }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl PostProcessor for PdfMetadataExtractor {
    async fn process(
        &self,
        result: &mut ExtractedDocument,
        _config: &ExtractionConfig,
    ) -> Result<()> {
        if result.mime_type != "application/pdf" {
            return Ok(());
        }

        let order = self.processed_count.fetch_add(1, Ordering::AcqRel) + 1;

        result.metadata.additional.insert("pdf_processed".to_string(), json!(true));
        result.metadata.additional.insert("pdf_order".to_string(), json!(order));
        result.metadata.additional.insert(
            "content_length".to_string(),
            json!(result.content.len()),
        );
        result.metadata.additional.insert(
            "pdf_processor_version".to_string(),
            json!("1.0.0"),
        );

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
    register_post_processor(Arc::new(PdfMetadataExtractor {
        processed_count: AtomicUsize::new(0),
    }))?;
    Ok(())
}
```
