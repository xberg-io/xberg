# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Minimum supported versions raised: Node.js 22, Ruby 3.3, PHP 8.3, Go 1.27, Java 25, Android API 24, Dart 3.13, macOS 26, Rust 1.99.

## [1.0.0-rc.32] - 2026-07-23

### Added

- Initial `@xberg-io/llamaindex-xberg` release with the `XbergReader` and `XbergNodeParser` for LlamaIndex.TS.
- `XbergReader` loads a single file, a list of files, or raw bytes via the `@xberg-io/xberg` native binding,
  emitting one `Document` per source (or per page when page splitting is enabled) with flattened metadata,
  tables, keywords, annotations, and forwarded element/chunk streams.
- `XbergNodeParser` splits reader documents into `TextNode`s along Xberg's native chunks or structural
  elements, excluding the forwarding payload from LLM and embedding input.
