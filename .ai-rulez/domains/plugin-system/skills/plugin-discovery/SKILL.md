---
name: plugin-discovery
---
Discover and load plugins dynamically

1. For static Rust plugins:
   a. Create plugin instance
   b. Validate implements traits
   c. Register with registry
2. For Python plugins:
   a. Scan module paths
   b. Import modules
   c. Find plugin classes
   d. Validate implements protocol
   e. Instantiate with PyO3
   f. Register with registry
3. Validate before registration:
   - Check required methods exist
   - Verify method signatures
   - Test initialization
4. Handle discovery errors gracefully
5. Report unregistered plugins
