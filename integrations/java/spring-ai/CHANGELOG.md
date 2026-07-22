# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-31

### Added

- Initial release of Xberg Spring AI DocumentReader
- Core `XbergDocumentReader` implementation with fluent Builder API
- Support for multiple input resource types:
  - `FileSystemResource` (optimized direct file extraction)
  - `ByteArrayResource` (in-memory content with explicit MIME type)
  - `ClassPathResource` (bundled application resources)
  - `UrlResource` (remote document retrieval)
- Four-level document splitting with automatic priority detection:
  - Native chunking with heading context (RAG-ready)
  - Element-based splitting (semantic document structure)
  - Page-based splitting (one Document per page)
  - Whole document (single Document with all content)
- Rich metadata mapping with 20+ explicit fields:
  - Document properties (title, subject, authors, keywords, timestamps)
  - Extraction metadata (language detection, quality score, output format)
  - Format-specific pass-through (PDF, Excel, Email, PowerPoint, Images, HTML, etc.)
  - Splitting-specific fields (chunk context, element types, bounding boxes)
- Comprehensive test suite:
  - 29 unit tests covering all code paths
  - 80%+ line coverage via JaCoCo
  - Mocked Xberg extraction for isolated testing
  - Validation and error handling verification
- Build tooling and quality gates:
  - Google Java Format via Spotless for code consistency
  - Checkstyle linting with Google checks configuration
  - JaCoCo coverage enforcement (80% minimum)
  - Maven Surefire with Java 25 preview flags
- CI/CD workflows for GitHub Actions:
  - Linting job (formatting, Checkstyle, pre-commit hooks)
  - Test job (unit tests with Java 25)
  - Publish job (Maven Central deployment on version tags)
  - Secrets configuration for GPG signing and OSSRH credentials
- Documentation:
  - Comprehensive README with quickstart examples and metadata reference
  - CHANGELOG (this file)
  - Inline Javadoc for public API
- Project configuration:
  - Maven `pom.xml` with Java 25 and preview flag configuration
  - MIT License
  - `.gitignore` for Maven/IDE artifacts
  - `.pre-commit-config.yaml` for commit-time linting

### Capabilities

- **100+ document format support** via Xberg:
  - Office documents (PDF, DOCX, XLSX, PPTX, ODT, ODS, ODP)
  - Web formats (HTML, XML, XHTML)
  - Structured formats (JSON, YAML, CSV)
  - Compressed archives (ZIP, TAR, GZ)
  - Images (PNG, JPG, GIF, WEBP, BMP)
  - Specialized formats (EML, MSG, DWG, CAD formats)

- **Native OCR with 80+ language support** for scanned documents and images

- **Advanced layout detection** using RT-DETR v2 for accurate text extraction and positioning

- **Table extraction** via TATR and SLANeXT models for structured data recovery

- **Zero external service dependencies** — all processing happens locally in-process using bundled native libraries

- **Spring AI integration** — seamless integration with Spring AI's `DocumentReader` interface and ETL pipeline

### Notes

- Requires Java 25 with `--enable-preview` flag due to Foreign Function & Memory API usage in Xberg
- Native libraries (~43 MB) are bundled in the JAR; zero additional Java dependencies beyond Jackson (included in Spring Boot)
- All extraction APIs are pass-through for future Xberg version compatibility
