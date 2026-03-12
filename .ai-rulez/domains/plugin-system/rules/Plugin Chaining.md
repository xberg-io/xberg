---
name: Plugin Chaining
priority: high
---
Support sequential post-processor execution

- Plugins applied in priority order
- Each plugin modifies result for next
- Failures don't prevent other plugins
- Track which plugins were applied
- Maintain result structure throughout chain
