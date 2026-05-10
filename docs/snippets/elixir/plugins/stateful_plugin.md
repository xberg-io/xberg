<!-- snippet:skip -->

Stateful plugin implementation is not available in the Elixir binding. Stateful plugins must be implemented in Rust using `Arc<Mutex<>>` or `Arc<RwLock<>>` for thread-safe state management.

To implement a stateful plugin in Rust:

```rust
use kreuzberg::plugins::{Plugin, PostProcessor};
use kreuzberg::{Result, ExtractionResult, ExtractionConfig};
use std::sync::{Arc, Mutex};
use async_trait::async_trait;

struct StatefulProcessor {
    state: Arc<Mutex<ProcessorState>>,
}

struct ProcessorState {
    extraction_count: usize,
}

impl Plugin for StatefulProcessor {
    fn name(&self) -> &str { "stateful-processor" }
    fn version(&self) -> String { "1.0.0".to_string() }
    fn initialize(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.extraction_count = 0;
        Ok(())
    }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl PostProcessor for StatefulProcessor {
    async fn process(
        &self,
        _result: &mut ExtractionResult,
        _config: &ExtractionConfig
    ) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.extraction_count += 1;
        Ok(())
    }
}
```

Register this in Rust and use it from Elixir.
