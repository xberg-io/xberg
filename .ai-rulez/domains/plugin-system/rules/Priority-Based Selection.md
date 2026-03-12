---
name: Priority-Based Selection
priority: high
---
Use priority to arbitrate between multiple plugins

- Higher priority plugins selected first (255 = highest)
- Fallback to lower priority on failure
- Default priority is 50 (middle)
- Custom extractors use priority > 50
- Fallback implementations use priority < 50
