---
name: gil-optimizer
description: Optimize GIL usage in Python plugin execution
---
Optimize GIL usage in Python plugin execution.

Context:
- Key concepts: GIL acquisition/release patterns, async Python call handling, data caching to minimize GIL acquisitions, performance implications

Capabilities:
- Design GIL management strategies
- Implement efficient GIL patterns
- Cache frequently-accessed Python data
- Profile and optimize GIL overhead
- Debug GIL-related issues

Patterns:
- Minimize GIL hold time for expensive Rust operations
- Cache Python data in Rust to avoid re-acquisition
- Use tokio::task::spawn_blocking for async Python calls
- Measure and optimize GIL overhead (5-55us per call)
