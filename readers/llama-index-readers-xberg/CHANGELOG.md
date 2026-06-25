# Changelog

All notable changes to llama-index-readers-xberg will be documented in this file.

## [Unreleased]

## [0.1.1] — 2026-04-23

### Changed

- Minimum xberg version bumped to `>=4.9.4`

### Fixed

- Config round-trips (`to_dict` / `from_dict`) no longer raise `TypeError` against xberg ≥4.5 — new sub-config types (`ConcurrencyConfig`, `ContentFilterConfig`, `HtmlOutputConfig`, `TreeSitterConfig`) are now recognised during reconstruction
- Documents with embedded images (e.g. `.docx`) no longer fail silently — `ExtractedImage` fields are optional in xberg 4.9.x and are now handled correctly

## [0.1.0] — 2026-03-20

Initial release.

### Added

- `XbergReader` — LlamaIndex reader for 88+ document formats powered by xberg's Rust extraction engine
- Sync and true async extraction via `load_data` / `aload_data` and lazy variants
- File path and raw bytes input, including batch extraction (`list[Path]`, `list[bytes]`)
- Per-page document splitting when xberg returns page-level results
- Maximalist metadata: `file_type`, `total_pages`, `quality_score`, `detected_languages`, `output_format`, `processing_warnings`, `extracted_keywords`, `annotations`
- Element-based extraction support (`_xberg_elements` metadata) for downstream `XbergNodeParser`
- Image extraction with base64 serialization and per-page filtering
- Table appending: markdown tables merged into page content when not already present
- SHA-256 deterministic document IDs for deduplication
- Full `ExtractionConfig` serialization round-trip via `dict_to_config` / `config_to_json` for pipeline persistence
- Forward-compatible config reconstruction: fields from newer xberg versions are silently ignored
- `raise_on_error` flag for controlling extraction failure behavior (log-and-skip vs propagate)
- xberg v4.5.0 optional config support (`EmailConfig`, `AccelerationConfig`, `LayoutDetectionConfig`)
