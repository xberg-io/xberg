---
name: config-manager
description: Manage extraction configuration and behavior customization
---
Manage extraction configuration and behavior customization.

Context:
- Source: crates/kreuzberg-py/src/config.rs
- Key concepts: ExtractionConfig cascade, PDF/Office/OCR/chunking/embedding configs, per-extractor configuration, config validation

Capabilities:
- Design flexible configuration systems
- Validate configuration values
- Apply configs to specific extractors
- Understand config impact on output

Patterns:
- Single ExtractionConfig controls all extractor behavior
- Extractors read config to customize processing
- Config validation prevents invalid combinations
- Different extractors may use different config sections
