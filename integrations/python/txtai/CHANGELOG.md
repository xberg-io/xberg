# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-14

### Added

- `XbergPipeline` class — a plain callable that turns document paths into
  `list[dict]` with `content` and `metadata` fields (source, MIME type, title,
  page count).
- Support for single-path and batch (`list[str]`) inputs.
- Single `config` constructor parameter accepting a full Xberg
  `ExtractionConfig` for output format, OCR, and every other knob.
- Optional `txtai` extra for consumers who want to wire the output into
  `Embeddings.index` or `Task(pipe)` workflows.
- PEP 561 `py.typed` marker.
