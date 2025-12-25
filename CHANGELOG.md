# Changelog

All notable changes to Kreuzberg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [4.0.0-rc.19] - 2025-12-25

### Added

- **Font configuration API** - Configurable font provider with custom directory support and automatic path expansion
- **Homebrew bottle support** - Pre-built macOS bottles for faster installation
- **Environment variable configuration** - `KREUZBERG_MAX_REQUEST_BODY_BYTES` and `KREUZBERG_MAX_MULTIPART_FIELD_BYTES` for API size limits
- **Config file caching** - Improved performance for TOML/YAML/JSON config file loading

### Fixed

- **Large file uploads** (issue #248) - Files larger than 2MB are now accepted (configurable up to 100MB)
- **Browser package Vite compatibility** (issue #249) - Fixed missing `pdfium.js` in dist bundle
- **Node.js missing binaries** (issue #241) - Fixed resolution in Docker and pnpm monorepo environments
- **Ruby gem native extension build** - Simplified build system with proper linker path resolution
- **Java E2E test compatibility** - Regenerated tests for Java 25
- **Docker ONNX Runtime** - Pinned to version 1.23 for compatibility
- **Font provider thread safety** - Fixed race condition and graceful lock poisoning handling
- **Custom font path validation** - Added security hardening with symlink resolution and canonicalization

### Changed

- **BREAKING**: Custom font provider now enabled by default (set `enabled = false` to disable)
- **Default API size limit** - Increased to 100MB (configurable via `KREUZBERG_MAX_UPLOAD_SIZE_MB`)
- **TypeScript serialization** - Replaced MessagePack + Base64 with direct JSON serialization
- **PDF dependency** - Temporarily using `kreuzberg-pdfium-render` fork while awaiting upstream PR merge

### Performance

- 15-25% overall execution improvement, 30-45% memory reduction
- String allocation optimization (-2.57% CPU, -0.81% memory)
- Memory pool improvements (35-50% reduction: 60-135 MB → 30-80 MB)
- Post-processing pipeline optimizations (10-17% CPU improvement)
- TypeScript thread pool dynamically sized by CPU count

### Removed

- **Legacy code cleanup** - Removed deprecated backward compatibility
  - TypeScript: `KREUZBERG_LEGACY_SERIALIZATION` environment variable
  - Go: 7 legacy error codes
  - Ruby: `Ocr = OCR` alias
  - Rust: Deprecated `Metadata.date` field
  - Cargo: 3 legacy feature aliases

### Security

- Custom font directories validated with canonicalization
- Symlinks resolved to prevent path traversal attacks
- All custom paths validated before use

## [4.0.0-rc.18] - 2025-12-23

### Fixed

- **Ruby gem packaging** - Fixed missing kreuzberg-ffi crate in vendored dependencies
- **Ruby gem macOS** - Fixed linker errors during gem build and installation
- **Python wheels macOS** - Fixed ImportError from hardcoded dylib paths

## [4.0.0-rc.17] - 2025-12-22

### Added

- **Docker ARM64 support** - Multi-architecture Docker images now support linux/arm64

### Fixed

- **Python wheels macOS** - Fixed ImportError when installing from wheel
- **Ruby gems macOS** - Fixed linker errors during gem installation
- **TypeScript plugin registration** - Fixed TypeError with JavaScript-style plugins

### Performance

- **Go bindings** - Improved ConfigMerge performance with native field copying

## [4.0.0-rc.16] - 2025-12-21

### Added

- **Batch processing APIs** - 4-6x throughput improvement for high-volume document extraction

### Fixed

- **Python IDE support** - Type stub files (`.pyi`) now included in wheel distributions
- **Java Maven compatibility** - Fixed CI builds with Maven 4.0.0-rc-4+ support
- **Go Windows linking** - Resolved duplicate linker flags causing compilation failures
- **Ruby gem compilation** - Fixed missing link search paths on Linux and Windows
- **Ruby gem publishing** - Fixed artifact corruption during publication process

### Performance

- **Batch operations** - 2-3x batch throughput gains with FFI streaming
- **C# optimizations** - JSON serialization with source generation
- **TypeScript/Node.js** - Config validation and batch operation integration

## [4.0.0-rc.15] - 2025-12-20

### Fixed

- **Node.js Windows publishing** - Windows x64 platform packages now publish correctly to npm

## [4.0.0-rc.14] - 2025-12-20

### Added

- **Comprehensive test suites** - End-to-end tests for all language bindings

### Fixed

- **C# NuGet publishing** - Switched to direct API key authentication
- **LibreOffice in Docker** - Updated to version 25.8.4
- **Python IDE type hints** - Type stub files now included in wheels
- **Ruby gem compilation** - Fixed Rust crate vendoring
- **Python ExtractionResult** - Fixed missing `pages` field in IDE autocomplete

## [4.0.0-rc.13] - 2025-12-19

### Fixed

- **PDF bundled feature flag** - Corrected flag to `bundled-pdfium`
- **Go Windows linking** - Fixed missing system libraries
- **Ruby gem packaging** - Added missing TOML dependency
- **WASM distribution** - Added compiled binaries for proper NPM publishing

## [4.0.0-rc.12] - 2025-12-19

### Fixed

- **Python wheels PDFium bundling** - Corrected conditional compilation feature flag
- **C# bindings** - Fixed MSBuild target for CI-downloaded native assets
- **Ruby bindings** - Added missing `unsafe` keyword for Rust 2024 edition
- **Docker** - Corrected ONNX Runtime package name for Debian Trixie

## [4.0.0-rc.11] - 2025-12-18

### Fixed

- **PDFium bundling** - Now correctly included in all language bindings
- **C# native libraries** - Build target properly copies platform-specific libraries
- **Ruby gem publishing** - Fixed validation errors from double-compression
- **Go Windows linking** - Removed duplicate CGO linker flags
- **WASM** - Added PDF extraction support for browser and Node.js

## [4.0.0-rc.10] - 2025-12-16

### Breaking Changes

- **PDFium feature names** - `pdf-static` → `static-pdfium`, `pdf-bundled` → `bundled-pdfium`, `pdf-system` → `system-pdfium`
- **Default PDFium linking** - `pdf` feature now defaults to `bundled-pdfium`
- **Go module path** - Moved to `github.com/kreuzberg-dev/kreuzberg/packages/go/v4` (update imports and run `go mod tidy`)

### Fixed

- **Windows CLI** - Now includes bundled PDFium runtime
- **WASM** - PDFium support for native targets
- **Go bindings** - Added `ExtractFileWithContext()` and batch variants
- **TypeScript** - Replaced `any` types with proper definitions
- **Ruby bindings** - Complete YARD documentation
- **C# bindings** - Complete XML documentation

## [4.0.0-rc.9] - 2025-12-15

### Added

- **`PDFIUM_STATIC_LIB_PATH` environment variable** - Enables custom static PDFium paths for Docker builds

### Fixed

- **Python** - Wheels now include typing metadata (`.pyi` stubs) for IDE support
- **Java** - Maven packages now bundle platform-specific native libraries
- **Node** - npm platform packages now contain compiled `.node` binaries
- **WASM** - Node.js runtime no longer crashes with `self is not defined`
- **PDFium static linking** - Fixed to correctly search for `libpdfium.a`

## [4.0.0-rc.8] - 2025-12-14

### Added

- **MCP HTTP Stream transport** - HTTP Stream transport for MCP server with SSE support

### Fixed

- **Go bindings** - Fixed CGO library path configuration for Linux and macOS
- **Python wheels** - Now built with correct manylinux compatibility
- **Ruby gems** - Removed embedding model cache from distribution
- **Maven Central** - Updated publishing to use modern Sonatype Central API

## [4.0.0-rc.7] - 2025-12-12

### Added

- **Configurable PDFium linking** - `pdf-static`, `pdf-bundled`, `pdf-system` Cargo features
- **WebAssembly bindings** - Full TypeScript API with sync/async extraction for browser, Cloudflare Workers, Deno
- **RTF extractor improvements** - Structured table extraction and metadata support
- **Page tracking redesign** - Byte-accurate page boundaries and per-page metadata

### Changed

- **BREAKING**: `ChunkMetadata` field renames (#226)
  - `char_start` → `byte_start`
  - `char_end` → `byte_end`
  - See migration guide for details

### Fixed

- **Ruby gem corruption** - Excluded embedding model cache from distribution
- **Java FFM SIGSEGV** - Fixed struct alignment on macOS ARM64
- **C# compilation errors** - Fixed variable shadowing
- **Python CI timeouts** - Marked slow office document tests for proper test selection

## [4.0.0-rc.6] - 2025-12-10

### Added

- **`core` feature** - Lightweight FFI build without ONNX Runtime (enables Windows MinGW compatibility)

### Fixed

- **ODT table extraction** - Fixed duplicate content extraction
- **ODT metadata extraction** - Enhanced to match Office Open XML capabilities
- **Go Windows MinGW builds** - Disabled embeddings feature for Windows compatibility
- **Ruby rb-sys conflict** - Removed vendoring, now uses crates.io version
- **Python text extraction metadata** - Fixed missing `format_type` field
- **C# E2E tests** - Fixed OCR tests with empty content

## [4.0.0-rc.5] - 2025-12-01

### Breaking Changes

- **Removed all Pandoc dependencies** - Native Rust extractors now handle all 12 previously Pandoc-supported formats
  - LaTeX, EPUB, BibTeX, Typst, Jupyter, FictionBook, DocBook, JATS, OPML, Org-mode, reStructuredText, RTF
  - Benefits: No system dependencies, smaller Docker images, pure Rust codebase

### Fixed

- **macOS CLI binary** - Fixed missing libpdfium.dylib at runtime
- **Windows Go builds** - Fixed GNU toolchain detection issues
- **Ruby Bundler 4.0** - Fixed gem installation failures

## [4.0.0-rc.4] - 2025-12-01

### Fixed

- **Publishing workflow** - Fixed crates.io and Maven Central authentication
- **Language bindings** - Resolved test failures across Ruby, Java, C#, and Node
- **ONNX Runtime** - Fixed mutex errors and deadlocks

## [4.0.0-rc.3] - 2025-12-01

### Fixed

- **NuGet publishing** - Switched to API key authentication
- **CLI binary packages** - Included libpdfium shared library

## [4.0.0-rc.2] - 2025-11-30

### Breaking Changes

- **TypeScript/Node.js package** - Renamed from `kreuzberg` to `@kreuzberg/node` (scoped package)
- **TypeScript migration** - Replace `import { ... } from 'kreuzberg'` with `import { ... } from '@kreuzberg/node'`

### Added

- **C#/.NET bindings** - Complete C# bindings using .NET 9+ FFM API
- **MkDocs documentation site** - Multi-language examples and API reference

### Fixed

- **Tesseract OCR** - Corrected API call ordering
- **Go Windows CGO linking** - Fixed MinGW compatibility
- **Python async tests** - Fixed pytest-asyncio configuration
- **Embeddings model cache** - Added lock poisoning recovery

## [4.0.0-rc.1] - 2025-11-23

### Major Release - Complete Rewrite

Complete architectural rewrite from Python-only to Rust-core with polyglot bindings.

### Architecture

- **Rust core** - All extraction logic implemented in Rust for performance
- **Polyglot bindings** - Python (PyO3), TypeScript/Node.js (NAPI-RS), Ruby (Magnus), Java (FFM API), Go (CGO)
- **10-50x performance improvements** - Streaming parsers for multi-GB files with constant memory

### Added

- **Plugin system** - PostProcessor, Validator, Custom OCR, Custom Document Extractors
- **Language detection** - Automatic multi-language detection
- **RAG & embeddings** - Automatic embedding generation with 4 presets (fast/balanced/quality/multilingual)
- **Image extraction** - Native extraction from PDFs and PowerPoint with metadata
- **Stopwords system** - 64 language support, compile-time embedded
- **Comprehensive metadata** - Format-specific extraction for PDF, Office, Email, Images, XML, HTML
- **MCP server** - Model Context Protocol integration for Claude
- **Docker support** - Multi-variant images with OCR backends
- **CLI improvements** - Rust-based CLI with better performance

### Changed

- **Async-first API** - Primary extraction functions are async (sync variants have `_sync` suffix)
- **Strongly-typed config** - All configuration uses typed structs/classes
- **Strongly-typed metadata** - Format-specific TypedDict/struct metadata
- **New API** - `extract()` → `extract_file()`, added `extract_bytes()`, `batch_extract_files()`

### Removed

- **Pure-Python API** - No longer available
- **Pandoc** - Native Rust extractors for all formats
- **GMFT** - Vision-based table extraction (replaced with Tesseract-based)
- **spaCy entity extraction** - Use external NER with postprocessors
- **KeyBERT** - Use external keyword extraction with postprocessors
- **Document classification** - Use external classifiers with postprocessors

### Breaking Changes

- Python 3.10+, Node.js 18+, Rust 1.75+ required
- Binary wheels only (no pure-Python installation)
- TypeScript/Node.js package renamed to `@kreuzberg/node`
- `char_start`/`char_end` → `byte_start`/`byte_end`

See [Migration Guide](https://docs.kreuzberg.dev/migration/v3-to-v4/) for details.

---

## [3.22.0] - 2025-11-27

### Fixed
- Always attempt EasyOCR import before raising `MissingDependencyError` to improve error handling
- Hardened HTML regexes for script/style stripping to prevent edge cases

### Added
- Test coverage for EasyOCR import path edge cases

### Changed
- Updated v3 dependencies to current versions

---

## [3.21.0] - 2025-11-05

### Major Release: Kreuzberg v4 Migration

This release introduces **Kreuzberg v4**, a complete rewrite with Rust core, polyglot bindings (Python/TypeScript/Ruby/Java/Go), and enhanced architecture. v3 remains available for legacy projects.

### Added

#### Core (Rust)
- Complete Rust core library with comprehensive document extraction pipeline
- Plugin system architecture for extensible extractors, OCR backends, postprocessors, and validators
- PDF extraction and rendering with advanced table detection
- Format extraction for Office documents (DOCX, XLSX, PPTX), HTML, XML, and plain text
- OCR subsystem with pluggable backend support (Tesseract, EasyOCR, PaddleOCR)
- Image processing utilities with EXIF extraction and format conversion
- Text processing with token reduction, chunking, keyword extraction, and language detection
- Stopwords utilities for 50+ languages
- Cache system with in-memory and persistent storage
- Embeddings support via FastEmbed
- MCP (Model Context Protocol) server integration
- CLI binary for command-line document extraction

#### Language Bindings
- **Python**: PyO3 FFI bindings with Python-idiomatic API and async support
- **TypeScript/Node.js**: NAPI-RS bindings with full type definitions
- **Ruby**: Magnus FFI bindings with RBS type definitions
- **Java**: Java 25 Foreign Function & Memory API (FFM/Panama) bindings
- **Go**: CGO bindings with Go 1.25+ support
- **C#**: FFI bindings (baseline implementation)

#### API Server & MCP
- REST API server for document extraction
- MCP server for Claude integration
- Comprehensive configuration system with builder patterns

#### Testing & Infrastructure
- 95%+ test coverage across all components
- End-to-end test suites for all language bindings (auto-generated from fixtures)
- Comprehensive Rust test suite with unit/integration/doc tests
- Multi-language CI/CD with GitHub Actions
- Docker builds for containerized deployment
- Benchmark harness for performance analysis

#### Documentation
- New documentation site: https://docs.kreuzberg.dev
- Language-specific guides for all 6+ bindings
- Plugin development documentation
- API reference with code examples
- Architecture documentation
- Migration guide from v3 to v4

### Changed

- Architecture restructured around Rust core with thin language-specific wrappers
- Build system upgraded to Rust Edition 2024 with Cargo workspace
- Dependency management via Cargo, npm/pnpm, PyPI, Maven, and Go modules
- Task automation via Taskfile.yaml
- Enhanced error handling with typed exceptions across all languages
- Configuration system redesigned for consistency across languages

### Removed

- Old v3 codebase superseded by v4 (v3 remains available in separate branch)
- Legacy Python implementation details replaced by PyO3 bindings
- Previous Node.js implementation replaced by NAPI-RS

### Security

- All dependencies audited for vulnerabilities
- Sandboxed subprocess execution (LibreOffice, Tesseract)
- Input validation on all user-provided data
- Memory safety via Rust

### Performance

- Streaming PDF extraction for memory efficiency
- Zero-copy patterns throughout Rust core
- SIMD optimizations where applicable
- ONNX Runtime for embeddings
- Async-first design for I/O operations

---

## [3.20.2] - 2025-10-11

### Fixed

- Surface missing optional dependency errors in GMFT extractor
- Stabilize aggregation data loading in benchmarks
- Make Docker E2E API checks resilient to transient failures

### Changed

- Make comparative benchmarks manual-only to reduce resource consumption

### Dependencies

- Updated dependencies to latest versions

---

## [3.20.1] - 2025-10-11

### Fixed

- Correct publish docker workflow include path

### Changed

- Optimize sdist size by excluding unnecessary files

---

## [3.20.0] - 2025-10-11

### Added

- Python 3.14 support

### Changed

- Migrate HTML extractor to html-to-markdown v2 for improved compatibility and performance

### Fixed

- Comparative benchmarks: fix frozen dataclass mutation and aggregation workflow
- CI improvements and coverage adjustments
- Add pytest-benchmark to dev dependencies for benchmark workflow

### Dependencies

- Bump astral-sh/setup-uv from 6 to 7
- Bump peter-evans/dockerhub-description from 4 to 5
- Bump actions/download-artifact from 4 to 5
- Bump actions/setup-python from 5 to 6
- Bump actions/checkout from 4 to 5

### Documentation

- Document Python 3.14 support limitations

---

## [3.19.1] - 2025-09-30

### Fixed

- Replace mocked Tesseract test with real file-based test
- Remove Rust build step from kreuzberg benchmarks workflow
- Resolve prek and mypy issues in comparative-benchmarks
- Add ruff config to comparative-benchmarks to allow benchmark patterns
- Ensure Windows Tesseract 5.5.0 HOCR output compatibility
- Properly type TypedDict configs with type narrowing and cast

### Changed

- Optimize extractor configurations for fair comparison in benchmarks
- Complete comparative-benchmarks workspace with visualization and docs generation

### Dependencies

- Updated dependencies to latest versions

### Testing

- Add regression test for Issue #149 Windows Tesseract HOCR compatibility

---

## [3.19.0] - 2025-09-29

### Added

- Enforce critical system error policy with context-aware exception handling
- Implement systematic exception handling audit across all modules
- Add German language pack support for Windows CI

### Fixed

- Align sync/async OCR pipelines and fix Tesseract PSM enum handling
- Remove magic library dependency completely
- Prevent magic import crashes in Windows tests
- Properly mock magic import in CLI tests
- Correct asyncio.gather result assertion in profiler test
- Resolve benchmark test failures and naming cleanup
- Add Windows-safe fallbacks for CLI progress and magic
- Make OCR test less brittle for character recognition
- Eliminate all Rich Console instantiations at import time
- Prevent Windows access violation in benchmarks CLI
- Update OCR test assertion to match actual error message format
- Handle ValidationError gracefully in batch processing contexts
- Correct test fixture file paths and remove hardcoded paths
- Ensure coverage job only runs when all tests pass

### Changed

- Remove verbose AI-pattern naming from components
- Enable coverage job for branches and update changelog

### Dependencies

- Add missing needs declaration to python-tests job in workflows

### Documentation

- Align documentation with v4-dev improvements
- Improve error handling policy documentation

### Performance

- Parallelize comparative benchmarks with 6-hour timeout

---

## [3.18.0] - 2025-09-27

### Added

- Make API server configuration configurable via environment variables
- Improve spaCy model auto-download with uv fallback
- Add regression tests for German image PDF extraction (issue #149)

### Changed

- Use uv to install prek for latest version
- Replace pre-commit commands with Prek in Taskfile.yml
- Update pre-commit instructions to use Prek
- Update html-to-markdown to latest version
- Auto-download missing spaCy models for entity extraction
- Fix HOCR parsing issues

### Fixed

- Resolve mypy type checking issues
- Prevent DeepSource upload when tests fail
- Make DeepSource upload optional when DEEPSOURCE_DSN is missing
- Prevent coverage-pr from running when test-pr fails

### Dependencies

- Remove pre-commit dependency from dev requirements
- Updated dependencies to latest versions

### CI/CD

- Use prek instead of pre-commit in validation workflow

### Documentation

- Update contributing guide to use prek instead of pre-commit
- Update uv.lock to reflect dependency changes and remove upload-time attributes

---

## [3.17.0] - 2025-09-17

### Added

- Add token reduction for text optimization
- Add concurrency settings to cancel in-progress workflow runs
- Optimize token reduction and update dependencies
- Optimize token reduction performance and add streaming support

### Fixed

- Resolve excessive markdown escaping in OCR output (fixes #133)
- Remove invalid table extraction tests for non-existent functions
- Ensure comprehensive test coverage in CI
- Resolve Windows path compatibility and improve test coverage
- Critical issues from second review of token reduction

### Changed

- Complete token reduction implementation overhaul

### Testing

- Improve coverage and add pragma no cover annotations

---

## [3.16.0] - 2025-09-16

### Added

- Enhance JSON extraction with schema analysis and custom field detection
- Add internal streaming optimization for html-to-markdown conversions
- Comprehensive test coverage improvements

### Fixed

- Export HTMLToMarkdownConfig in public API
- Resolve EasyOCR module-level variable issues and adjust coverage requirement
- Resolve mypy type errors and test failures
- Add xfail markers to chunking, ML-dependent, cache, and language detection tests
- Add xfail markers for EasyOCR device validation tests
- Resolve CI test failures with targeted fixes
- Add missing Any import for type annotations
- Update docker e2e workflow to use correct test filename
- Prevent docker_e2e.py from being discovered by pytest
- Resolve Windows-specific path issues in tests

### Changed

- Remove coverage fail_under threshold completely
- Remove unnecessary xfail markers

### Dependencies

- Bump actions/download-artifact from 4 to 5

### Testing

- Add comprehensive tests for API configuration caching
- Increase test coverage to meet CI requirements

---

## [3.15.0] - 2025-09-14

### Added

- Add comprehensive image extraction support
- Add polars DataFrame and PIL Image serialization for API responses
- Improve test coverage across core modules

### Fixed

- Resolve mypy type errors in CI
- Remove unused flame_config parameter from profile_benchmark
- Resolve CI formatting issues by ignoring generated AGENTS.md
- Resolve pre-commit formatting issues and ruff violations
- Resolve all test failures and achieve full test suite compliance
- Add polars DataFrame and PIL Image serialization for API responses
- Resolve TypeError unhashable type dict in API config merging
- Address linting and type issues from PR #130 review

### Changed

- Apply ruff formatting across the codebase

### Documentation

- Add comprehensive image extraction documentation
- Update documentation to use ImageOCRConfig instead of deprecated fields

### Cleanup

- Remove INTERFACE_PARITY.md causing mdformat issues

---

## [3.14.0] - 2025-09-13

### Added

- Comprehensive DPI configuration system for OCR processing with fine-grained control over resolution settings

### Changed

- Enhanced API with 1GB upload limit and comprehensive OpenAPI documentation
- Completed pandas to polars migration across entire codebase for improved performance and memory efficiency

### Fixed

- Improved lcov coverage combining robustness in CI pipeline
- DPI configuration tests moved to proper test directory for correct CI coverage calculation
- Pre-commit formatting issues resolved
- PDF content handling differences between CI and local tests
- PaddleOCR test isolation and fixture path corrections

---

## [3.13.0] - 2025-09-04

### Added

- Runtime configuration API with query parameters and header support for flexible request handling
- Comprehensive runtime configuration documentation with practical examples
- OCR caching system for EasyOCR and PaddleOCR backends to improve performance

### Changed

- Replaced pandas with polars for table extraction (significant performance improvement)
- Consolidated benchmarks CLI into unified interface with improved structure
- Converted all class-based tests to function-based tests for consistency
- Restructured benchmarks package layout for better organization
- Removed all module-level docstrings from Python files for cleaner codebase

### Fixed

- MyPy errors and type annotations throughout codebase
- Tesseract TSV output format and table extraction implementation
- UTF-8 encoding handling across document processing pipeline
- Config loading with proper error handling and validation
- HTML-to-Markdown configuration now externalized for flexibility
- All ruff and mypy linting errors resolved
- Test failures in CI environment
- Regression in PDF extraction and XLS file handling
- Subprocess error analysis for CalledProcessError handling
- Empty DataFrames in GMFT table extraction (pandas.errors.EmptyDataError prevention)

### Performance

- Optimized PDF processing and HTML conversion performance
- Improved TSV integration test quality with concrete assertions

---

## [3.12.0] - 2025-08-30

### Added

- Docker E2E testing infrastructure with multi-image matrix strategy
- Multilingual OCR support to Docker images with flexible backend selection
- Docker documentation with clarity on image contents, sizes, and OCR backend selection

### Changed

- Simplified Docker images to base and core variants (removed intermediate images)
- Docker strategy updated to 2-image approach for optimized distribution
- Docker E2E tests aligned with CI patterns using matrix strategy for parallel execution
- Improved Docker workflow with manual trigger options for flexibility

### Fixed

- Docker image naming conventions (kreuzberg-core:latest)
- Boolean condition syntax in Docker workflow
- Docker permissions and documentation accuracy
- Docker E2E test failure detection (ensures proper test exit code propagation)
- Docker workflow disk space management and optimization
- Grep exit code handling in Docker test workflows
- Naming conflict in CLI config command
- grep exit code failures in shell scripts
- EasyOCR test image selection (use text-containing image instead of flower)
- Test fixture image file naming for clarity

### Infrastructure

- Enhanced disk space cleanup in Docker E2E workflow
- Improved Docker workflow disk space management
- Updated test-docker-builds.yml for new image naming scheme

---

## [3.11.1] - 2025-08-13

### Fixed

- EasyOCR device-related parameters removed from readtext() calls
- Numpy import optimization: only import numpy inside process_image_sync rather than at module level to improve startup time
- Table attribute access in documentation
- Pre-commit formatting for table documentation

### Infrastructure

- Updated uv.lock after version bump
- AI-rulez pre-commit hooks compatibility with Go setup in CI workflow
- Temporary ai-rulez hooks disabled to unblock CI
- Reverted to ai-ruyles v1.4.3 with Go setup for CI stability
- Dependency updates including amannn/action-semantic-pull-request v5→v6 and actions/checkout v4→v5

---

## [3.11.0] - 2025-08-01

### Added

- Comprehensive tests for Tesseract OCR backend with edge case coverage
- Comprehensive tests for API module with full functionality coverage
- Comprehensive tests for config and image extractor modules
- Comprehensive Click-based tests for CLI module with proper command testing
- Comprehensive tests for structured data extractor module
- Comprehensive tests for extraction and document classification modules
- Comprehensive tests for pandoc extractor module
- Comprehensive tests for email extractor module
- Comprehensive tests for _config.py module
- Comprehensive tests for _utils._errors module with error handling validation
- Comprehensive tests for spreadsheet metadata extraction
- Retry mechanisms for CI test reliability to handle transient failures

### Changed

- Implemented Python 3.10+ syntax optimizations (match/case, union types where applicable)
- Merged comprehensive test files with regular test files for consolidated coverage
- Optimized pragma no cover usage to reduce false negatives
- Converted test classes to functional tests for better pytest compatibility
- Updated coverage requirements documentation from 95% to 85%

### Fixed

- Linting issues in merged comprehensive test files
- Image extractor async delegation tests
- CI test failures with improved retry mechanisms
- Timezone assertion in spreadsheet metadata test
- MyPy errors in test files and config calls
- ExceptionGroup import errors for Python 3.10+ compatibility
- Syntax errors in test files
- Bug in _parse_date_string with proper test mocks
- Pre-commit formatting issues
- DeepSource coverage reporting with restructured CI workflow

### Infrastructure

- Updated uv.lock to revision 3 to fix CI validation issues
- Fixed CI workflow structure for improved DeepSource coverage reporting
- Coverage upload now only triggers on push events (not pull requests)
- Version bump to 3.10.1 with lock file updates

---

## [3.10.0] - 2025-07-29

### Added

- PDF password support through new crypto extra feature

### Documentation

- Updated quick start section to use available configuration options

---

## [3.9.0] - 2025-07-17

### Added

- Initial release of v3.9.0 series with foundational features

---

## [3.8.0] - 2025-07-16

### Added

- Foundation for v3.8.0 release

---

## [3.7.0] - 2025-07-11

### Added

- MCP server for AI integration enabling Claude integration with document extraction capabilities
- MCP server configuration and optional dependencies support

### Fixed

- Chunk parameters adjusted to prevent overlap validation errors
- MCP server linting issues resolved
- HTML test compatibility with html-to-markdown v1.6.0 behavior

### Changed

- MCP server tests converted to function-based format
- MCP server configuration documentation clarified

---

## [3.6.0] - 2025-07-04

### Added

- Language detection functionality integrated into extraction pipeline

### Fixed

- Entity extraction migration from gliner to spaCy completed
- Docker workflow reliability improved with updated ai-rulez configuration
- Optional imports refactored to use classes for better type handling
- Validation and post-process logic abstracted into helper functions for sync and async paths

### Changed

- spaCy now used for entity extraction replacing gliner
- Optional imports moved inside functions for proper error handling
- Entity and keyword extraction validation/post-processing refactored

---

## [3.5.0] - 2025-07-04

### Added

- Language detection functionality with configurable backends
- Performance optimization guidelines and documentation
- Full synchronous support for PaddleOCR and EasyOCR backends
- Docker documentation and Python 3.10+ compatibility
- Benchmarks submodule integration

### Fixed

- Flaky OCR tests marked as xfail in CI environments
- Typing errors resolved across codebase
- Chunking default configuration reverted to stable settings
- PaddleOCR sync implementation updated to correct v3.x API format
- Python 3.9 support dropped and version bumped to v3.4.2
- Docker workflow version detection and dependencies fixed
- Docs workflow configuration and Docker manual dispatch enabled
- CI documentation dependencies resolved

### Changed

- Default configurations optimized for modern document extraction
- Tesseract OCR configuration optimized based on benchmark analysis
- Documentation link moved to top of README
- Python 3.10+ now required (3.9 support dropped)

---

## [3.4.0] - 2025-07-03

### Added

- API support with Litestar framework for web-based document extraction
- API tests aligned with best practices
- EasyOCR and GMFT Docker build variants
- Docker support with comprehensive CI integration
- API and Docker documentation

### Fixed

- 'all' extra dependency configuration simplified
- Docker workflow extras configuration corrected
- Race condition in GMFT caching tests resolved
- Race condition in Tesseract caching tests resolved
- Docker Hub repository configuration fixed
- CLI integration test using tmp_path fixture for isolation
- gmft module mypy type errors resolved

### Changed

- Docker module migrated to api module with Litestar framework
- Docker workflow matrix simplified
- Performance documentation enhanced with competitive benchmarks

---

## [3.3.0] - 2025-06-23

### Added

- Isolated process wrapper for GMFT table extraction
- Comprehensive statistical benchmarks proving msgpack superiority
- Comprehensive CLI support with Click framework
- Comprehensive benchmarking suite for sync vs async performance
- Pure synchronous extractors without anyio dependencies
- Document-level caching with simplified concurrency handling
- Per-file locks and parallel batch processing for sync performance
- Python version matrix CI testing (3.9-3.13)
- Thread lock for pypdfium2 to prevent macOS segfaults

### Fixed

- All remaining mypy and pydoclint issues ensuring CI passes
- Python 3.13 mock compatibility issues in process pool tests
- ExceptionGroup compatibility in sync tests
- GMFT config updated and test failures resolved
- All Windows-specific test failures in multiprocessing and utils modules
- Windows compatibility issues resolved
- Test coverage improvements from 76% to 83%
- Rust formatting issues with edition 2024 let-chain syntax
- File existence validation added to extraction functions
- Python linting issues (ruff complexity, mypy type parameters) resolved
- All clippy warnings resolved
- C# generated file formatting fixed

### Changed

- msgspec JSON replaced with msgpack for 5x faster cache serialization
- Caching migrated from msgspec JSON to msgpack for improved performance
- Async performance optimized with semaphore-based concurrency control
- Sync performance optimized with per-file locks and parallel batch processing
- Code formatting cleanup and non-essential comments removed
- Benchmark workflows removed from CI
- Python version cache key updated to include full version

---

## [3.2.0] - 2025-06-23

### Added

- GPU acceleration support for enhanced OCR and ML operations

### Fixed

- EasyOCR byte string issues resolved
- Pandoc version issues fixed
- PaddleOCR configuration updated for optimal performance
- Multiple language support added to EasyOCR

### Changed

- playa-pdf pinned to version 0.4.3 for stability
- PaddleOCR configuration optimized
- Dependencies updated to latest compatible versions
- Pandoc version string resolution improved

---

## [3.1.0] - 2025-03-28

### Added

- GMFT (Give Me Formatted Tables) support for vision-based table extraction

### Fixed

- Bundling issue corrected
- Wrong link in README fixed
- Dependencies updated and GMFT testing issues resolved

### Changed

- Image extraction now non-optional in results
- Test imports and structure updated
- Concurrency test added for validation

---

## [3.0.0] - 2025-03-23

### Added

- Chunking functionality for document segmentation
- Extractor registry for managing format-specific extractors
- Hooks system for pre/post-processing
- OCR backend abstraction with EasyOCR and PaddleOCR support
- OCR configuration system
- Multiple language support in OCR backends
- Comprehensive documentation

### Fixed

- Pre-commit hooks configuration
- Windows error message handling
- PaddleOCR integration issues
- Dependencies updated to stable versions

### Changed

- Structure refactored for improved organization
- Metadata extraction approach updated
- OCR integration with configurable backends
- Documentation added and improved

---

## See Also

- [Configuration Reference](reference/configuration.md) - Detailed configuration options
- [Migration Guide](migration/v3-to-v4.md) - v3 to v4 migration instructions
- [Format Support](reference/formats.md) - Supported file formats
- [Extraction Guide](guides/extraction.md) - Extraction examples

[4.0.0-rc.7]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.6...v4.0.0-rc.7
[4.0.0-rc.6]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.5...v4.0.0-rc.6
[4.0.0-rc.5]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.4...v4.0.0-rc.5
[4.0.0-rc.4]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.3...v4.0.0-rc.4
[4.0.0-rc.3]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.2...v4.0.0-rc.3
[4.0.0-rc.2]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.1...v4.0.0-rc.2
[4.0.0-rc.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.0...v4.0.0-rc.1
[4.0.0-rc.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.21.0...v4.0.0-rc.0
[3.22.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.21.0...v3.22.0
[3.21.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.21.0
[3.20.2]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.20.1...v3.20.2
[3.20.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.20.0...v3.20.1
[3.20.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.19.1...v3.20.0
[3.19.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.19.0...v3.19.1
[3.19.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.18.0...v3.19.0
[3.18.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.17.0...v3.18.0
[3.17.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.16.0...v3.17.0
[3.16.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.15.0...v3.16.0
[3.15.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.14.0...v3.15.0
[3.14.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.13.0...v3.14.0
[3.13.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.12.0...v3.13.0
[3.12.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.11.1...v3.12.0
[3.11.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.11.0...v3.11.1
[3.11.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.10.0...v3.11.0
[3.10.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.9.0...v3.10.0
[3.9.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.8.0...v3.9.0
[3.8.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.7.0...v3.8.0
[3.7.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.6.0...v3.7.0
[3.6.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.5.0...v3.6.0
[3.5.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.4.0...v3.5.0
[3.4.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.3.0...v3.4.0
[3.3.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.2.0...v3.3.0
[3.2.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.1.0...v3.2.0
[3.1.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.0.0...v3.1.0
[3.0.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.0.0
