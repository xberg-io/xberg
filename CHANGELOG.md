# Changelog

All notable changes to xberg-surrealdb will be documented in this file.

## [0.2.0] — 2026-05-15

> **Migration required** — records ingested with `0.1.x` have different field shapes in SurrealDB. Drop and re-ingest your document and chunk tables after upgrading to ensure consistent data.

### Fixed

- **`authors` field** — stored as a comma-separated string instead of a raw list, matching the xberg `metadata.authors: list[str]` type.
- **`detected_languages` field** — now always a list (empty list instead of `None` when no languages were detected). SurrealQL queries checking `IF detected_languages = NONE` should be updated to check `array::len(detected_languages) = 0`.
- **`keywords` field** — now a `list[str]` of keyword text values instead of raw `ExtractedKeyword` objects. Application code that accessed `.text` on keyword items should use the string directly.
- **Chunk `char_start` / `char_end`** — these fields are always `None` (xberg does not expose character offsets). `page_number` on a chunk now maps to the chunk's `first_page` value.

### Schema changes

The SurrealDB field type definitions have changed to match the values actually stored. If you have an existing schema from `0.1.x`, run `REMOVE TABLE documents; REMOVE TABLE chunks;` and call `setup_schema()` again before re-ingesting.

- **`documents.authors`** — schema type changed from `option<array<string>>` to `option<string>`. The field now holds a comma-separated string, not an array.
- **`documents.keywords`** — schema type changed from `option<array<object>>` to `option<array<string>>`. The field now holds an array of plain keyword strings.

## [0.1.1] — 2026-03-13

### Changed

- Bump minimum xberg dependency from `>=4.4.4` to `>=4.4.6`
  - Better PDF extraction for positioned/tabular text (v4.4.5, #431)
  - DOCX image placeholder fix (v4.4.6, #484)
  - 13 additional file formats: dBASE (.dbf), HWP (.hwp/.hwpx), Office templates (.docm, .dotx, .dotm, .dot, .potx, .potm, .pot, .xltx, .xlt)

## [0.1.0] — 2026-03-08

Initial release.

### Added

- `DocumentConnector` — full-document extraction and BM25 search (no chunking or embeddings)
- `DocumentPipeline` — chunked extraction with local embeddings, hybrid search (vector + BM25 via RRF), and vector search
- `DatabaseConfig` / `IndexConfig` — configuration dataclasses for connection and index tuning
- Four ingestion methods: `ingest_file`, `ingest_files`, `ingest_directory`, `ingest_bytes`
- SHA-256 content-hash deduplication via deterministic record IDs
- Quality filtering on search results via `quality_threshold` parameter
- Detection of SurrealDB's silent INSERT IGNORE errors (dimension mismatch, etc.)
- Support for all xberg embedding presets: `"fast"`, `"balanced"`, `"quality"`, `"multilingual"`
- Async context manager lifecycle for both classes
