---
name: Conflict Resolution
priority: high
---
Resolve conflicts between plugins claiming same capability

- Multiple extractors can support same MIME type
- Highest-priority extractor attempted first
- Failed extractor triggers fallback
- Priority determines arbitration consistently
- Document priority-based decisions in logs
