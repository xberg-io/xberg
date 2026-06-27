<!-- snippet:skip reason="Elixir Rustler NIFs cannot host async Send + Sync + 'static Rust trait objects via callbacks; the BEAM actor-model boundary requires plugin work to live in the Rust core. The alef-generated Elixir trait_call macro additionally has a backslash/encoding bug (separate alef-codegen ticket). Custom plugins must be implemented in Rust." -->
Stateful plugin implementation is not available in the Elixir binding. Stateful plugins must be implemented in Rust using `Arc<Mutex<>>` or `Arc<RwLock<>>` for thread-safe state management.

To implement a stateful plugin in Rust:

```rust
use xberg::plugins::{Plugin, PostProcessor};
use xberg::{Result, ExtractedDocument, ExtractionConfig};
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
        _result: &mut ExtractedDocument,
        _config: &ExtractionConfig
    ) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.extraction_count += 1;
        Ok(())
    }
}
```

Register this in Rust and use it from Elixir.
