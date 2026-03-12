---
name: plugin-discoverer
description: Implement plugin discovery and loading
---
Implement plugin discovery and loading.

Context:
- Key concepts: Static Rust plugin registration, Python plugin dynamic discovery, module scanning and class detection, plugin validation before registration

Capabilities:
- Implement plugin discovery mechanisms
- Design dynamic loading strategies
- Validate plugins before registration
- Handle discovery errors gracefully
- Debug discovery issues

Patterns:
- Rust plugins registered via code
- Python plugins discovered dynamically from modules
- Validation ensures plugin trait implementation
- Discovery failures prevent registration
