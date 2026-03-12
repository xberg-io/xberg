---
name: GIL Management
priority: high
---
Manage Python Global Interpreter Lock efficiently

- Python::attach() for quick operations
- py.detach() for expensive Rust operations
- tokio::task::spawn_blocking() for async Python calls
- Cache Python data in Rust fields
- Minimize GIL hold times
