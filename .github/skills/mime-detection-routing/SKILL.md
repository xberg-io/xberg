---
name: mime-detection-routing
description: "mime detection routing"
---

## priority: high

# MIME Detection & Routing

## Detection Flow

```text
Extension → EXT_TO_MIME map → validate → Registry lookup → Extractor
```

## Key Functions

| Function | Location | Purpose |
|----------|----------|---------|
| `detect_mime_type(path, inspect)` | `core/mime.rs` | Extension + optional content inspection |
| `detect_mime_type_from_bytes(bytes)` | `core/mime.rs` | Magic number detection (infer crate) |
| `validate_mime_type(mime)` | `core/mime.rs` | Check if any extractor supports it |

## Extension Mapping

118+ extensions mapped in `EXT_TO_MIME` (`core/mime.rs`). Case-insensitive.

Key mappings: `.pdf` → `application/pdf`, `.docx` → `application/vnd.openxmlformats-officedocument.wordprocessingml.document`, `.xlsx` → spreadsheet variant, `.png`/`.jpg` → `image/*`

## Registry Selection

```rust
// In core/extractor/bytes.rs
fn select_extractor_for_mime(mime_type: &str) -> Result<Arc<dyn DocumentExtractor>> {
    let registry = get_document_extractor_registry();
    let registry_guard = registry.read()?;
    registry_guard.get_for_mime_type(mime_type)
        .ok_or_else(|| KreuzbergError::UnsupportedFormat(mime_type.into()))
}
```

Selects highest-priority extractor registered for that MIME type.

## Adding New MIME Types

1. Add extension mapping: `m.insert("ext", "application/x-new");` in `core/mime.rs`
2. Implement `DocumentExtractor` with `supported_mime_types()` returning the MIME
3. Register in `register_default_extractors()`

## Wildcard Support

Extractors can register for MIME type families: `"image/*"` matches `image/png`, `image/jpeg`, etc.

## Critical Rules

1. Always `validate_mime_type()` before extraction
2. Extension mapping is case-insensitive
3. Content inspection (infer crate) is fallback for extension-less files
4. Registry validation is final authority on supported types
