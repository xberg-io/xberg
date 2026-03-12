---
name: Telemetry & Observability
priority: high
---
Record extraction telemetry for debugging and optimization

- Use OpenTelemetry spans for extraction operations
- Record file format, size, duration
- Include error details in error events
- Sanitize file paths (filename only, no full path)
- Track cache hits/misses
