---
name: Memory Efficiency
priority: high
---
Minimize memory footprint during extraction

- Stream large PDFs when possible
- Don't load entire documents into memory
- Clear temporary buffers promptly
- Use Arc for shared data structures
- Monitor memory usage in long-running operations
