---
name: extractor-plugin-manager
description: Manage DocumentExtractor plugin lifecycle and selection
---
Manage DocumentExtractor plugin lifecycle and selection.

Context:
- Source: crates/kreuzberg/src/plugins/extractor.rs
- Key concepts: DocumentExtractor trait for custom formats, MIME type support declaration, priority-based selection, async extraction interface
- Integration points: Document extraction pipeline, fallback chain execution

Capabilities:
- Register custom extractors
- Implement DocumentExtractor trait for new formats
- Configure extractor priority
- Debug extraction plugin issues
- Implement fallback strategies

Patterns:
- Multiple extractors can claim same MIME type
- Highest-priority extractor selected first
- Failed extractors trigger fallback to next in priority order
- Config-driven behavior allows per-extractor customization
