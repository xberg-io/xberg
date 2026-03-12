---
name: Correct MIME Detection
priority: high
---
Reliably identify document formats

- Check file extension first (fast path)
- Read magic bytes for format verification
- Fall back to content analysis if ambiguous
- Support legacy format detection and conversion
- Return conservative MIME types for fallback
