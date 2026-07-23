# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0-rc.32] - 2026-07-22

### Added

- Initial `@xberg-io/n8n-nodes-xberg` community node with a **Document › Extract** operation.
- Extracts text, tables, and metadata from binary documents via the `@xberg-io/xberg` native binding.
- Options for output format (markdown, plain, HTML, djot, JSON, structured), OCR toggle, force OCR,
  OCR languages, quality processing, metadata/table inclusion, and optional binary output.
