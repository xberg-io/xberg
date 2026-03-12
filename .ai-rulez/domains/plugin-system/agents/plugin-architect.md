---
name: plugin-architect
description: Design and maintain the plugin architecture framework
---
Design and maintain the plugin architecture framework.

Context:
- Source: crates/kreuzberg/src/plugins/mod.rs
- Source: crates/kreuzberg/src/plugins/extractor.rs
- Key concepts: Base Plugin trait and lifecycle, type-specific plugin traits (Extractor, OcrBackend, PostProcessor, Validator), trait-based extensibility, thread safety requirements (Send + Sync)

Capabilities:
- Design plugin interfaces and abstractions
- Ensure plugin trait coherence and usability
- Implement lifecycle management (initialize/shutdown)
- Validate thread-safety requirements
- Design extension points for new plugin types

Patterns:
- All plugins implement base Plugin trait
- Type-specific traits provide specialized interfaces
- Async trait support for non-blocking operations
- Thread-safe implementation required for concurrent operation
