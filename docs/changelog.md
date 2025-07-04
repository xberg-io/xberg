# Changelog

All notable changes to Kreuzberg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Documentation site with comprehensive examples and API reference
- Improved configuration for all OCR backends
- Added hooks system for validation and post-processing
- Language detection feature with `auto_detect_language` configuration option
- New optional dependency group `langdetect` for automatic language detection

### Changed

- Refactored internal structure for better maintainability
- Updated extraction functions to use config object instead of kwargs
- Improved error messages and reporting

## [0.1.0] - 2023-11-15

### Added

- Initial release
- Support for PDF, image, and Office document extraction
- OCR capabilities with Tesseract
- Basic error handling and reporting
- Async and sync APIs
