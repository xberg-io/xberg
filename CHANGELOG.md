# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.2] - 2026-03-13

### Changed

- Bumped minimum kreuzberg version from `>=4.3.0` to `>=4.4.6`, bringing major extraction quality improvements and new format support
- Loader now supports 88+ formats. Thanks to kreuzberg!

## [1.0.1] - 2026-03-04

### Fixed

- Batch error detection now correctly checks `metadata["error"]` instead of `not result.metadata`, which never triggered in kreuzberg v4.x (metadata always contains `extraction_duration_ms`)
- `ProcessingWarning` objects in batch results are now serialized as `{source, message}` dicts instead of `str()` coercion, preserving structured data

## [1.0.0] - 2026-02-24

### Changed

- Moved repository to [kreuzberg-dev](https://github.com/xberg-io/langchain-kreuzberg) organization
- Updated copyright to Kreuzberg.dev
- Production-ready release

### Added

- Kreuzberg logo and centralized badge layout in README
- PyPI metadata: keywords, classifiers, documentation URL
- `CHANGELOG.md` included in sdist

## [0.1.0] - 2026-02-23

### Added

- `KreuzbergLoader` class extending `langchain_core.document_loaders.BaseLoader`
- Support for 75+ file formats via Kreuzberg extraction API
- Synchronous loading via `load()` and `lazy_load()` methods
- Native async loading via `aload()` and `alazy_load()` backed by Rust's tokio runtime
- File path input supporting single files, lists of files, and directories with glob patterns
- Raw bytes input for loading from API responses, S3 objects, and other in-memory sources
- Rich metadata extraction including title, author, page count, quality score, detected languages, and extracted keywords
- Table extraction with cell data and Markdown representation
- Per-page splitting mode yielding one `Document` per page for RAG pipelines
- OCR support with three configurable backends: Tesseract, EasyOCR, and PaddleOCR
- Output format selection: plain text, Markdown, Djot, HTML, and structured
- Full `ExtractionConfig` override for advanced configuration
- Type annotations and `py.typed` marker for static analysis

[1.0.2]: https://github.com/xberg-io/langchain-kreuzberg/releases/tag/v1.0.2
[1.0.1]: https://github.com/xberg-io/langchain-kreuzberg/releases/tag/v1.0.1
[1.0.0]: https://github.com/xberg-io/langchain-kreuzberg/releases/tag/v1.0.0
[0.1.0]: https://github.com/xberg-io/langchain-kreuzberg/releases/tag/v0.1.0
