---
name: Configuration Cascade
priority: high
---
Use single ExtractionConfig for all extraction behavior control

- All extractors respect ExtractionConfig settings
- Config changes don't require code modifications
- Different extractors may use different config sections
- Validate config values before extraction
- Document config impact on output
