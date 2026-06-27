```rust title="Rust"
impl PostProcessor for PdfOnlyProcessor {
    async fn process(
        &self,
        result: &mut ExtractedDocument,
        _config: &ExtractionConfig
    ) -> Result<()> {
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
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
