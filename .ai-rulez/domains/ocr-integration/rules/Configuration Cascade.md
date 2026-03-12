---
name: Configuration Cascade
priority: high
---
Apply OCR configuration consistently

- TesseractConfig controls all OCR behavior
- ImagePreprocessingConfig controls preprocessing
- OcrConfig cascades from ExtractionConfig
- Config overrides work hierarchically
- Validate config values
