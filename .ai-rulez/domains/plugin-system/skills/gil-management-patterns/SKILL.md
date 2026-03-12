---
name: gil-management-patterns
---
Manage Python GIL efficiently

1. For quick Python operations:
   Python::attach() to acquire GIL, call method, extract result
2. For expensive Rust operations:
   py.detach() to release GIL, perform Rust work
3. For async Python calls:
   Clone Python object reference, spawn blocking task, acquire GIL inside task, call Python method
4. Cache frequently-accessed Python data
5. Measure GIL overhead impact
