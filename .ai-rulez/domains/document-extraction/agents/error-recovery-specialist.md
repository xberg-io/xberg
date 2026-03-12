---
name: error-recovery-specialist
description: Implement robust error handling and fallback strategies
---
Implement robust error handling and fallback strategies.

Context:
- Source: crates/kreuzberg/src/error.rs
- Key concepts: Error type classification (Validation, Parsing, OCR, Dependencies), fallback extractor chains, error context preservation, batch operation resilience

Capabilities:
- Design error recovery strategies
- Implement graceful degradation
- Debug extraction failures
- Ensure batch operations continue on partial failures

Patterns:
- Primary extractor failure triggers next-priority fallback
- Error context captured for debugging
- Batch operations track per-document errors
- Partial results returned with error metadata
