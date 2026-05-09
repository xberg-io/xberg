---
title: "Error Reference"
---

## Error Reference

All error types thrown by the library across all languages.

### KreuzbergError

Main error type for all Kreuzberg operations.

All errors in Kreuzberg use this enum, which preserves error chains
and provides context for debugging.

# Variants

- `Io` - File system and I/O errors (always bubble up)
- `Parsing` - Document parsing errors (corrupt files, unsupported features)
- `Ocr` - OCR processing errors
- `Validation` - Input validation errors (invalid paths, config, parameters)
- `Cache` - Cache operation errors (non-fatal, can be ignored)
- `ImageProcessing` - Image manipulation errors
- `Serialization` - JSON/MessagePack serialization errors
- `MissingDependency` - Missing optional dependencies (tesseract, etc.)
- `Plugin` - Plugin-specific errors
- `LockPoisoned` - Mutex/RwLock poisoning (should not happen in normal operation)
- `UnsupportedFormat` - Unsupported MIME type or file format
- `Other` - Catch-all for uncommon errors

| Variant | Message | Description |
|---------|---------|-------------|
| `Io` | IO error: {0} | Io errors |
| `Parsing` | Parsing error: {message} | Parsing errors |
| `Ocr` | OCR error: {message} | Ocr errors |
| `Validation` | Validation error: {message} | Validation errors |
| `Cache` | Cache error: {message} | Cache errors |
| `ImageProcessing` | Image processing error: {message} | Image processing errors |
| `Serialization` | Serialization error: {message} | Serialization errors |
| `MissingDependency` | Missing dependency: {0} | Missing dependency errors |
| `Plugin` | Plugin error in '{plugin_name}': {message} | Plugin errors |
| `LockPoisoned` | Lock poisoned: {0} | Lock poisoned errors |
| `UnsupportedFormat` | Unsupported format: {0} | Unsupported format errors |
| `Embedding` | Embedding error: {message} | Embedding errors |
| `Timeout` | Extraction timed out after {elapsed_ms}ms (limit: {limit_ms}ms) | Timeout errors |
| `Cancelled` | Extraction cancelled | Cancelled errors |
| `Security` | Security violation: {message} | Security errors |
| `Other` | {0} | Other errors |

---
