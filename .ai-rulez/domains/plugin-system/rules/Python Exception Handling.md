---
name: Python Exception Handling
priority: high
---
Translate Python exceptions to Rust errors

- Catch PyException and convert to KreuzbergError
- Include Python error message in Rust error
- Log Python tracebacks for debugging
- Handle import errors gracefully
- Document expected Python exceptions
