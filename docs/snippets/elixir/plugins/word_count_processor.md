<!-- snippet:skip -->

Custom post-processor implementation is not available in the Elixir binding. Post-processors must be implemented in Rust using the `PostProcessor` trait.

To implement a word count processor in Rust:

```rust
use kreuzberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
use async_trait::async_trait;

struct WordCountProcessor;

impl Plugin for WordCountProcessor {
    fn name(&self) -> &str { "word-count" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl PostProcessor for WordCountProcessor {
    async fn process(
        &self,
        result: &mut ExtractionResult,
        _config: &ExtractionConfig
    ) -> Result<()> {
        let word_count = result.content.split_whitespace().count();
        // Store word count in metadata or processing warnings
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Early
    }

    fn should_process(
        &self,
        result: &ExtractionResult,
        _config: &ExtractionConfig
    ) -> bool {
        !result.content.is_empty()
    }
}
```

Register this processor in Rust and it will be applied during extraction in Elixir.
